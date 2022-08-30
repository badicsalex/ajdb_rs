// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::{identifier::ActIdentifier, reference::Reference, structure::Act, util::date::Date};

pub struct Database {}

// Should be somewhat lightweight
#[derive(Clone)]
pub struct DatabaseState {}

// Should contain cached data, and gie back act when needed
pub struct ActInDatabase {}

impl ActInDatabase {
    // Also does the save to db
    // This is where article dedup could be implemented
    pub fn save(act: Act) -> Self{
        todo!()
    }

    // This is what does the actual load
    pub fn load(&self) -> Act { todo!()}

    // Something about being in force? Probably cached
    // Other cached stuff and metadata here.
}

impl Database {
    pub fn new() -> Self {
        todo!()
    }

    pub fn get_state(&self, date: Date) -> DatabaseState {
        todo!()
    }
    pub fn set_state(&mut self, date: Date, state: DatabaseState) -> DatabaseState {
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

    pub fn get_acts_enforced_at_date(&self, date: Date) -> Vec<ActInDatabase> {
        todo!()
    }

    pub fn set_act(&mut self, act: ActInDatabase) {
        todo!()
    }
}

pub fn cli_add_raw(path: &str) {
    // Load act
    // Add to database at publish date
}

struct DateRange {}

impl DateRange {
    pub fn new(from: Date, to: Date) -> Self{
        todo!()
    }

}

impl Iterator for DateRange {
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub fn cli_recalculate(from: Date, to: Date) {
    let mut db = Database::new();
    let mut state = db.get_state(from);
    for date in DateRange::new(from.succ().unwrap(), to) {
        let new_state = db.get_state(date);
        for act in new_state.get_new_acts_compared_to(&state) {
            state.set_act(act);
        }
        for act in state.get_acts_enforced_at_date(date) {
            apply_amendments(&mut state, act, date);
        }
        db.set_state(date, state.clone());
    }
}

fn apply_amendments(state: &mut DatabaseState, act: ActInDatabase, date: Date) {
    todo!()
}
