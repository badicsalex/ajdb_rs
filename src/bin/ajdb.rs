// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::Path;

use anyhow::Result;

use ajdb::{amender::apply_amendments, database::{Database, ActInDatabase}, util::{NaiveDateRange, read_all}};
use chrono::NaiveDate;
use hun_law::structure::Act;

pub fn cli_add_raw(path: &Path) -> Result<()>{
    let act: Act = serde_yaml::from_slice(&read_all(path)?)?;
    let date = act.publication_date;
    let mut db = Database::new();
    let mut state = db.get_state(date);
    state.set_act(ActInDatabase::save(act));
    db.set_state(date, state);
    Ok(())
}
pub fn cli_recalculate(from: NaiveDate, to: NaiveDate) {
    let mut db = Database::new();
    let mut state = db.get_state(from);
    for date in NaiveDateRange::new(from.succ(), to) {
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

fn main() -> Result<()> {
    Ok(())
}
