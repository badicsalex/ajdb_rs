// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::any::Any;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io::Write, path::PathBuf};

use anyhow::Result;
use anyhow::{anyhow, Context};
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::cache_backend::CacheBackend;

/// Gzipped JSON-based persistence module
pub struct Persistence {
    persistence_dir: PathBuf,
    cache: CacheBackend<PersistenceKey, Arc<dyn Any + Send + Sync>>,
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
    pub fn store<T: serde::Serialize>(
        &self,
        input_key: KeyType,
        data: &T,
    ) -> Result<PersistenceKey> {
        // TODO: Use cache
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

    pub fn load<T: serde::de::DeserializeOwned>(&self, key: &PersistenceKey) -> Result<T> {
        // TODO: Use cache
        // TODO: Use readers throughout the body instead of buffers
        let file_path = self.path_for(key);
        let gz_encoded_data = fs::read(file_path)?;

        let mut gz_decoder = GzDecoder::new(Vec::new());
        gz_decoder.write_all(&gz_encoded_data)?;
        let the_json = gz_decoder.finish()?;

        Ok(serde_json::from_slice(&the_json)?)
    }

    /// Load the data and cache it. Important: the cache is not updated on
    /// store() calls, by design.
    pub async fn load_cached<T>(&self, key: &PersistenceKey) -> Result<Arc<T>>
    where
        T: serde::de::DeserializeOwned + Send + Sync + Any,
    {
        let result = self
            .cache
            .get_or_try_init::<anyhow::Error>(key.clone(), async move {
                let loaded = self.load::<T>(key)?;
                let the_arc: Arc<dyn Any + Send + Sync> = Arc::new(loaded);
                Ok(the_arc)
            })
            .await?;
        result
            .downcast()
            .map_err(|_| anyhow!("Invalid type in cache at key {key}"))
    }

    pub fn exists(&self, key: &PersistenceKey) -> Result<bool> {
        // The self.cache.contains() call assumes that files are not
        // deleted at runtime, because load() may fail in that case.
        Ok(self.cache.contains(key) || self.path_for(key).exists())
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
