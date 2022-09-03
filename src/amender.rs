// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};

use crate::database::{ActEntry, DatabaseState};

pub struct ActModification {}

impl ActModification {
    pub fn modify_act(&self, act: &mut Act) -> Result<()> {
        let _ = act;
        todo!()
    }
}

pub type ActModificationSet = HashMap<ActIdentifier, Vec<ActModification>>;

pub fn get_all_modifications(
    act_entries: &[ActEntry],
    date: NaiveDate,
) -> Result<ActModificationSet> {
    let result = HashMap::new();
    for act_entry in act_entries {
        let act = act_entry.act()?;
        let _ = act;
        let _ = date;
        // todo!()
    }
    Ok(result)
}

pub fn apply_all_modifications(
    state: &mut DatabaseState,
    modifications: &ActModificationSet,
) -> Result<()> {
    for (act_id, modifications) in modifications {
        let mut act = state.get_act(*act_id)?.act()?;
        for modification in modifications {
            modification.modify_act(&mut act)?;
        }
        state.store_act(act)?;
    }
    Ok(())
}
