// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{fs, io::Write, path::PathBuf};

use anyhow::Context;
use anyhow::Result;
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

pub struct Persistence {
    persistence_dir: PathBuf,
}

pub type PersistenceKey = String;

#[derive(Debug)]
pub enum KeyType {
    Forced(PersistenceKey),
    Calculated(&'static str),
}

impl Persistence {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Persistence {
            persistence_dir: path.into(),
        }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.persistence_dir.join(format!("{}.json.gz", key))
    }

    fn compute_key(prefix: &str, data: &[u8]) -> PersistenceKey {
        format!("{}/{:08x}", prefix, seahash::hash(data))
    }

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

        let key = match input_key {
            KeyType::Forced(key) => key,
            KeyType::Calculated(prefix) => Self::compute_key(prefix, &the_json),
        };

        // TODO: Skip this step if key is Calculated and data exists.
        let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gz_encoder
            .write_all(&the_json)
            .with_context(|| anyhow::anyhow!("Compression failed for {}", key))?;
        let gz_encoded_data = gz_encoder
            .finish()
            .with_context(|| anyhow::anyhow!("Compression finish failed for {}", key))?;

        let file_path = self.path_for(&key);
        if let Some(file_dir) = file_path.parent() {
            fs::create_dir_all(file_dir)
                .with_context(|| anyhow::anyhow!("Creating directories failed for {}", key))?;
        }
        fs::write(file_path, gz_encoded_data)
            .with_context(|| anyhow::anyhow!("Writing file data failed for {}", key))?;
        Ok(key)
    }

    pub fn load<T: serde::de::DeserializeOwned>(&self, key: &PersistenceKey) -> Result<T> {
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
}
