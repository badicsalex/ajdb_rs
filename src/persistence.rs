// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

use std::any::Any;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io::Write, path::PathBuf};

use anyhow::{anyhow, Context};
use anyhow::{ensure, Result};
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::cache_backend::CacheBackend;

/// Gzipped JSON-based persistence module
pub struct Persistence {
    persistence_dir: PathBuf,
    cache: CacheBackend<PersistenceKey, Arc<dyn Any + Send + Sync>>,
}

impl std::fmt::Debug for Persistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Persistence")
            .field("persistence_dir", &self.persistence_dir)
            .finish()
    }
}

pub type PersistenceKey = String;

#[derive(Debug)]
pub enum KeyType {
    /// Use a specific persistence key. Should include any prefixes
    Forced(PersistenceKey),
    /// Calculate persistence key from the hash of stored data.
    /// Use the included &str as a prefix (without trailing slash)
    Calculated(&'static str),
}

impl Persistence {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Persistence {
            persistence_dir: path.into(),
            cache: CacheBackend::new(NonZeroUsize::new(1024).unwrap()),
        }
    }

    /// Atomically store data at key. Reentrant, but order between concurrent saves is not guaranteed.
    pub fn store<T>(&self, input_key: KeyType, data: &T) -> Result<PersistenceKey>
    where
        T: serde::Serialize + Clone + Send + Sync + Any,
    {
        let the_json = serde_json::to_vec_pretty(data).with_context(|| {
            anyhow!(
                "Encoding to JSON failed for {:?}, value type={}",
                input_key,
                std::any::type_name::<T>()
            )
        })?;

        let key = match &input_key {
            KeyType::Forced(key) => key.clone(),
            KeyType::Calculated(prefix) => Self::compute_key(prefix, &the_json),
        };

        self.cache.set(key.clone(), Arc::new(data.clone()));

        let file_path = self.path_for(&key);

        if matches!(input_key, KeyType::Calculated(_)) && file_path.exists() {
            return Ok(key);
        }

        // TODO: Use writers from this part down.
        //       (Note that we caannot use a writer for the json part because we
        //       need the hash for the filename in the most common case)

        // TODO: Skip this step if key is Calculated and data exists.
        let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gz_encoder
            .write_all(&the_json)
            .with_context(|| anyhow!("Compression failed for {}", key))?;
        let gz_encoded_data = gz_encoder
            .finish()
            .with_context(|| anyhow!("Compression finish failed for {}", key))?;

        if let Some(file_dir) = file_path.parent() {
            fs::create_dir_all(file_dir)
                .with_context(|| anyhow!("Creating directories failed for {}", key))?;
        }
        Self::atomic_write(&file_path, &gz_encoded_data)
            .with_context(|| anyhow!("Writing file data failed for {}", key))?;
        Ok(key)
    }

    fn load_from_disk<T>(&self, key: &PersistenceKey) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: Use readers throughout the body instead of buffers
        let file_path = self.path_for(key);
        let gz_encoded_data = fs::read(file_path)?;

        let mut gz_decoder = GzDecoder::new(Vec::new());
        gz_decoder.write_all(&gz_encoded_data)?;
        let the_json = gz_decoder.finish()?;

        Ok(serde_json::from_slice(&the_json)?)
    }

    pub fn load<T>(&self, key: &PersistenceKey) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    {
        if let Some(result) = self.cache.get(key) {
            if let Ok(result) = result.downcast::<T>() {
                return Ok((*result).clone());
            }
        }
        self.load_from_disk(key)
    }

    /// The efficient version of load()
    pub async fn load_async<T>(&self, key: &PersistenceKey) -> Result<Arc<T>>
    where
        T: serde::de::DeserializeOwned + Send + Sync + Any,
    {
        let result = self
            .cache
            .get_or_try_init::<anyhow::Error>(key.clone(), async move {
                let loaded = self.load_from_disk::<T>(key)?;
                let the_arc: Arc<dyn Any + Send + Sync> = Arc::new(loaded);
                Ok(the_arc)
            })
            .await?;
        result
            .downcast()
            .map_err(|_| anyhow!("Invalid type in cache at key {key}"))
    }

    pub fn exists(&self, key: &PersistenceKey) -> Result<bool> {
        Ok(self.cache.contains(key) || self.path_for(key).exists())
    }

    pub fn is_link(&self, key: &PersistenceKey) -> Result<bool> {
        Ok(self.cache.contains(key) || self.path_for(key).is_symlink())
    }

    pub fn link(&self, from: &PersistenceKey, to: &PersistenceKey) -> Result<()> {
        let from_path = self.path_for(from);
        ensure!(
            from_path.exists(),
            "Error linking {from} to {to}: file does not exist"
        );
        let to_path = self.path_for(to);
        if to_path.exists() {
            fs::remove_file(&to_path)?
        }
        let to_path_parent = to_path
            .parent()
            .ok_or_else(|| anyhow!("{to_path:?} is not in a directory"))?;
        fs::create_dir_all(to_path_parent)
            .with_context(|| anyhow!("Creating directories failed for {to}"))?;
        std::os::unix::fs::symlink(
            pathdiff::diff_paths(
                fs::canonicalize(&from_path)?,
                fs::canonicalize(to_path_parent)?,
            )
            .ok_or_else(|| {
                anyhow!("Could not compute relative path for {from_path:?} to {to_path:?}")
            })?,
            to_path,
        )?;
        // TODO: cache
        Ok(())
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.persistence_dir.join(format!("{}.json.gz", key))
    }

    fn compute_key(prefix: &str, data: &[u8]) -> PersistenceKey {
        let hash: u64 = seahash::hash(data);
        format!(
            "{}/{:02x}/{:06x}",
            prefix,
            hash >> 56,
            hash & 0xFFFFFFFFFFFFFF
        )
    }

    fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
        let file_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let name = path
            .file_name()
            .ok_or_else(|| anyhow!("No filename found in atomic_write: {:?}", path))?;
        fs::create_dir_all(&file_dir)?;
        let mut tmp_fil = tempfile::Builder::new()
            .prefix(name)
            .suffix(".tmp")
            .tempfile_in(&file_dir)?;
        tmp_fil.write_all(bytes)?;
        tmp_fil.flush()?;
        tmp_fil.persist(path)?;
        Ok(())
    }
}
