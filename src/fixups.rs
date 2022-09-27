// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{fs::File, path::PathBuf};

use anyhow::Result;
use hun_law::{identifier::ActIdentifier, semantic_info::EnforcementDate, util::singleton_yaml};
use serde::{Deserialize, Serialize};

use crate::amender::AppliableModification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fixup {
    AddModification(AppliableModification),
    AddEnforcementDate(EnforcementDate),
}

#[derive(Debug, Clone)]
pub struct Fixups {
    fixups: Vec<Fixup>,
}

impl Fixups {
    pub fn load(act_id: ActIdentifier) -> Result<Self> {
        Self::load_from(act_id, "./data/fixups/".into())
    }

    pub fn load_from(act_id: ActIdentifier, base_dir: PathBuf) -> Result<Self> {
        let fixup_path = base_dir
            .join(act_id.year.to_string())
            .join(format!("{}.yml", act_id));
        let fixups = if fixup_path.exists() {
            singleton_yaml::from_reader(File::open(&fixup_path)?)?
        } else {
            Vec::new()
        };
        Ok(Self { fixups })
    }

    pub fn get_additional_modifications(&self) -> Vec<AppliableModification> {
        self.fixups
            .iter()
            .filter_map(|f| {
                if let Fixup::AddModification(m) = f {
                    Some(m.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_additional_enforcement_dates(&self) -> Vec<EnforcementDate> {
        self.fixups
            .iter()
            .filter_map(|f| {
                if let Fixup::AddEnforcementDate(m) = f {
                    Some(m.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
