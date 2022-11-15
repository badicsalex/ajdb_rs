// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{fs::File, path::PathBuf};

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, semantic_info::EnforcementDate, util::singleton_yaml};
use serde::{Deserialize, Serialize};

use crate::amender::AppliableModification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActFixup {
    AddModification(AppliableModification),
    AddEnforcementDate(EnforcementDate),
}

#[derive(Debug, Clone)]
pub struct ActFixups {
    fixups: Vec<ActFixup>,
}

impl ActFixups {
    pub fn load(act_id: ActIdentifier) -> Result<Self> {
        Self::load_from(act_id, "./data/fixups/act/".into())
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
                if let ActFixup::AddModification(m) = f {
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
                if let ActFixup::AddEnforcementDate(m) = f {
                    Some(m.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GlobalFixup {
    AddModification(AppliableModification),
}

#[derive(Debug, Clone)]
pub struct GlobalFixups {
    fixups: Vec<GlobalFixup>,
}

impl GlobalFixups {
    pub fn load(date: NaiveDate) -> Result<Self> {
        Self::load_from(date, "./data/fixups/date/".into())
    }

    pub fn load_from(date: NaiveDate, base_dir: PathBuf) -> Result<Self> {
        let fixup_path = base_dir.join(date.format("%Y-%m-%d.yml").to_string());
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
            .map(|f| match f {
                GlobalFixup::AddModification(modification) => modification.clone(),
            })
            .collect()
    }
}
