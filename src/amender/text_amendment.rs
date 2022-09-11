// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier, reference::Reference, semantic_info::TextAmendmentReplacement,
    structure::Act,
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedTextAmendment {
    pub position: Reference,
    pub replacement: TextAmendmentReplacement,
}

impl Modify<Act> for SimplifiedTextAmendment {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
}

impl AffectedAct for SimplifiedTextAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))
    }
}
