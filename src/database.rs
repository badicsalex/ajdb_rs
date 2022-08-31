// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::PathBuf;

use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, reference::Reference, structure::Act};

#[derive(Debug, Clone)]
pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

pub struct Database {
    persistence: Persistence,
}

pub struct DatabaseState {
    persistence: Persistence,
}

// Should contain cached data, and gie back act when needed
pub struct StoredAct {
    persistence: Persistence,
}

impl Database {
    pub fn new(persistence: Persistence) -> Self {
        todo!()
    }

    pub fn get_state(&self, date: NaiveDate) -> DatabaseState {
        todo!()
    }
    pub fn set_state(&mut self, date: NaiveDate, state: DatabaseState) {
        todo!()
    }

    /// Copy acts from old_date state to new_date state,
    /// overwriting exisitng acts and keeping new ones.
    pub fn copy_state(&mut self, old_date: NaiveDate, new_date: NaiveDate) {
        todo!()
    }
}

impl DatabaseState {
    pub fn act(&self, id: ActIdentifier) -> StoredAct {
        todo!()
    }

    pub fn get_new_acts_compared_to(&self, other: &DatabaseState) -> Vec<StoredAct> {
        todo!()
    }

    pub fn get_acts_enforced_at_date(&self, date: NaiveDate) -> Vec<StoredAct> {
        todo!()
    }

    pub fn store_act(&mut self, act: Act) -> StoredAct {
        todo!()
    }
}

impl StoredAct {
    // This is what does the actual load
    pub fn load(&self) -> Act {
        todo!()
    }

    // Something about being in force? Probably cached
    // Other cached stuff and metadata here.
}
