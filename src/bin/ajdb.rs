// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.
use anyhow::Result;

use ajdb::{amender::apply_amendments, database::Database, util::NaiveDateRange};
use chrono::NaiveDate;

pub fn cli_add_raw(path: &str) {
    // Load act
    // Add to database at publish date
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
