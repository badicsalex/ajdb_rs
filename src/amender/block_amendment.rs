// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::{semantic_info::BlockAmendment, structure::BlockAmendmentChildren};

use super::ModifyAct;

#[derive(Debug)]
pub struct BlockAmendmentWithContent {
    pub block_amendment: BlockAmendment,
    pub content: BlockAmendmentChildren,
}

impl ModifyAct for BlockAmendmentWithContent {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
