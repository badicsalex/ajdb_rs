// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{identifier::ActIdentifier, reference::Reference, structure::Act};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedRepeal {
    pub position: Reference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Modify<Act> for SimplifiedRepeal {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!();
    }
}

impl AffectedAct for SimplifiedRepeal {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (Repeal)"))
    }
}
