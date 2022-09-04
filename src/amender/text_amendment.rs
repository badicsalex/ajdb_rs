// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::semantic_info::TextAmendment;

use super::ModifyAct;

impl ModifyAct for TextAmendment {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
