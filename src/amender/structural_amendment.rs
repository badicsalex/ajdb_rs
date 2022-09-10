// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    reference::structural::StructuralReference,
    structure::{Act, ActChild},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralBlockAmendmentWithContent {
    pub position: StructuralReference,
    pub pure_insertion: bool,
    pub content: Vec<ActChild>,
}

impl Modify<Act> for StructuralBlockAmendmentWithContent {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
}

impl AffectedAct for StructuralBlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act.ok_or_else(|| {
            anyhow!("No act in reference in special phrase (StructuralBlockAmendmentWithContent))")
        })
    }
}
