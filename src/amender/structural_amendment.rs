// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::{semantic_info::StructuralBlockAmendment, structure::ActChild};

use super::ModifyAct;

#[derive(Debug)]
pub struct StructuralBlockAmendmentWithContent {
    pub block_amendment: StructuralBlockAmendment,
    pub content: Vec<ActChild>,
}

impl ModifyAct for StructuralBlockAmendmentWithContent {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
