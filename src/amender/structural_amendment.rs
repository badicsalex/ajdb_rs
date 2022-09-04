// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::{semantic_info::StructuralBlockAmendment, structure::ActChild};

use super::ModifyAct;

pub struct StructuralBlockAmendmentWithContent {
    block_amendment: StructuralBlockAmendment,
    content: Vec<ActChild>,
}

impl ModifyAct for StructuralBlockAmendmentWithContent {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
