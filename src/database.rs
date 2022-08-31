// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, reference::Reference, structure::Act};

pub struct Database {}

// Should be somewhat lightweight
#[derive(Clone)]
pub struct DatabaseState {}

// Should contain cached data, and gie back act when needed
pub struct ActInDatabase {}

impl ActInDatabase {
    // Also does the save to db
    // This is where article dedup could be implemented
    pub fn save(act: Act) -> Self {
        todo!()
    }

    // This is what does the actual load
    pub fn load(&self) -> Act {
        todo!()
    }

    // Something about being in force? Probably cached
    // Other cached stuff and metadata here.
}

impl Database {
    pub fn new() -> Self {
        todo!()
    }

    pub fn get_state(&self, date: NaiveDate) -> DatabaseState {
        todo!()
    }
    pub fn set_state(&mut self, date: NaiveDate, state: DatabaseState) -> DatabaseState {
        todo!()
    }
}

impl DatabaseState {
    pub fn get_act(&self, id: ActIdentifier) -> ActInDatabase {
        // Maybe return a lighter weight Act, or Act Proxy
        todo!()
    }

    pub fn get_new_acts_compared_to(&self, other: &DatabaseState) -> Vec<ActInDatabase> {
        todo!()
    }

    pub fn get_acts_enforced_at_date(&self, date: NaiveDate) -> Vec<ActInDatabase> {
        todo!()
    }

    pub fn set_act(&mut self, act: ActInDatabase) {
        todo!()
    }
}
