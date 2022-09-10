// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    reference::Reference,
    structure::{Act, BlockAmendmentChildren},
};
use serde::{Deserialize, Serialize};

use super::ModifyAct;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAmendmentWithContent {
    pub position: Reference,
    pub pure_insertion: bool,
    pub content: BlockAmendmentChildren,
}

impl ModifyAct for BlockAmendmentWithContent {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act().ok_or_else(|| {
            anyhow!("No act in reference in special phrase (BlockAmendmentWithContent)")
        })
    }
}
