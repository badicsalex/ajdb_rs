// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::{semantic_info::BlockAmendment, structure::BlockAmendmentChildren};

use super::ModifyAct;

pub struct BlockAmendmentWithContent {
    block_amendment: BlockAmendment,
    content: BlockAmendmentChildren,
}

impl ModifyAct for BlockAmendmentWithContent {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
