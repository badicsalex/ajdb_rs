// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};

use crate::database::{DatabaseState, StoredAct};

pub struct ActModification {}

impl ActModification {
    pub fn affected_act_id(&self) -> ActIdentifier {
        todo!()
    }

    pub fn modify_act(&self, act: Act) -> Act {
        todo!()
    }
}

pub fn get_all_modifications(acts: &[StoredAct], date: NaiveDate) -> Vec<ActModification> {
    todo!()
}

pub fn apply_all_modifications(state: &mut DatabaseState, modifications: &[ActModification]) {
    for modification in modifications {
        let act = state.act(modification.affected_act_id()).load();
        let act = modification.modify_act(act);
        state.store_act(act);
    }
}
