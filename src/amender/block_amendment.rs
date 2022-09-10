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

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAmendmentWithContent {
    pub position: Reference,
    pub pure_insertion: bool,
    pub content: BlockAmendmentChildren,
}

impl Modify<Act> for BlockAmendmentWithContent {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
}

impl AffectedAct for BlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act().ok_or_else(|| {
            anyhow!("No act in reference in special phrase (BlockAmendmentWithContent)")
        })
    }
}
