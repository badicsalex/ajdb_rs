// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{fs, io::Write, path::PathBuf};

use anyhow::Context;
use anyhow::Result;
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Gzipped JSON-based persistence module
pub struct Persistence {
    persistence_dir: PathBuf,
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
        }
    }

    // NOTE: This function needs &mut self even though nothing is actually mutated.
    //       this is to signify that the fs in fact is mutated and this operation
    //       should probably be protected somehow.
    //       Also if we did a fully in-memory storage, we would need &mut anyway.
    pub fn store<T: serde::Serialize>(
        &mut self,
        input_key: KeyType,
        data: &T,
    ) -> Result<PersistenceKey> {
        let the_json = serde_json::to_vec_pretty(data).with_context(|| {
            anyhow::anyhow!(
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
            .with_context(|| anyhow::anyhow!("Compression failed for {}", key))?;
        let gz_encoded_data = gz_encoder
            .finish()
            .with_context(|| anyhow::anyhow!("Compression finish failed for {}", key))?;

        if let Some(file_dir) = file_path.parent() {
            fs::create_dir_all(file_dir)
                .with_context(|| anyhow::anyhow!("Creating directories failed for {}", key))?;
        }
        fs::write(file_path, gz_encoded_data)
            .with_context(|| anyhow::anyhow!("Writing file data failed for {}", key))?;
        Ok(key)
    }

    // TODO: Caching, probably the deserialized struct. Or at least the decompressed json.
    pub fn load<T: serde::de::DeserializeOwned>(&self, key: &PersistenceKey) -> Result<T> {
        // TODO: Use readers throughout the body instead of buffers
        let file_path = self.path_for(key);
        let gz_encoded_data = fs::read(file_path)?;

        let mut gz_decoder = GzDecoder::new(Vec::new());
        gz_decoder.write_all(&gz_encoded_data)?;
        let the_json = gz_decoder.finish()?;

        Ok(serde_json::from_slice(&the_json)?)
    }

    pub fn exists(&self, key: &PersistenceKey) -> Result<bool> {
        Ok(self.path_for(key).exists())
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
}
