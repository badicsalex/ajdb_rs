// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::semantic_info::StructuralRepeal;

use super::ModifyAct;

impl ModifyAct for StructuralRepeal {
    fn modify_act(&self, act: &mut hun_law::structure::Act) -> anyhow::Result<()> {
        todo!()
    }
}
