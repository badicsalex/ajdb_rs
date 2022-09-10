// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    semantic_info::StructuralBlockAmendment,
    structure::{Act, ActChild},
};

use super::ModifyAct;

#[derive(Debug)]
pub struct StructuralBlockAmendmentWithContent {
    pub metadata: StructuralBlockAmendment,
    pub content: Vec<ActChild>,
}

impl ModifyAct for StructuralBlockAmendmentWithContent {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.metadata
            .position
            .act
            .ok_or_else(|| anyhow!("No act in reference in special phrase"))
    }
}
