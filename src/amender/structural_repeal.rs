// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{identifier::ActIdentifier, semantic_info::StructuralRepeal, structure::Act};

use super::ModifyAct;

impl ModifyAct for StructuralRepeal {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act
            .ok_or_else(|| anyhow!("No act in reference in special phrase (StructuralRepeal)"))
    }
}
