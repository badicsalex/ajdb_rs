// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{identifier::ActIdentifier, semantic_info::TextAmendment, structure::Act};

use super::ModifyAct;

impl ModifyAct for TextAmendment {
    fn apply(&self, _act: &mut Act) -> Result<()> {
        todo!()
    }
    fn affected_act(&self) -> Result<ActIdentifier> {
        let result = self
            .positions
            .first()
            .ok_or_else(|| anyhow!("No positions in special phrase (TextAmendment)"))?
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))?;
        ensure!(
            self.positions.iter().all(|p| p.act() == Some(result)),
            "The positions didn't correspond to the same act (TextAmendment)"
        );
        Ok(result)
    }
}
