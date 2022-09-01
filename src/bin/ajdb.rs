// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::Path;

use anyhow::Result;

use ajdb::{
    amender::{apply_all_modifications, get_all_modifications},
    database::Database,
    persistence::Persistence,
    util::{read_all, NaiveDateRange},
};
use chrono::NaiveDate;
use hun_law::structure::Act;

pub fn cli_add_raw(path: &Path) -> Result<()> {
    let act: Act = serde_yaml::from_slice(&read_all(path)?)?;
    let date = act.publication_date;
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    let mut state = db.get_state(date)?;
    state.store_act(act)?;
    state.save()?;
    Ok(())
}
pub fn cli_recalculate(from: NaiveDate, to: NaiveDate) -> Result<()> {
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    for date in NaiveDateRange::new(from.succ(), to) {
        db.copy_state(date.pred(), date)?;
        let mut state = db.get_state(date)?;
        let interesting_acts = state.get_acts_enforced_at_date(date);
        let modifications = get_all_modifications(&interesting_acts, date)?;
        apply_all_modifications(&mut state, &modifications)?;
        state.save()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    Ok(())
}
