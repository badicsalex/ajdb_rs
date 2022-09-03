// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use ajdb::{
    amender::{apply_all_modifications, get_all_modifications},
    database::Database,
    persistence::Persistence,
    util::NaiveDateRange,
};
use anyhow::Result;
use chrono::NaiveDate;
use log::info;

#[derive(Debug, clap::Args)]
pub struct RecalculateArgs {
    /// Starting date (inclusive)
    // TODO: Automatic from, based on the first non-empty state
    from: NaiveDate,
    /// Ending date (exclusive)
    // TODO: Automatic to, based on the last enforcement date
    to: NaiveDate,
}

pub fn cli_recalculate(args: RecalculateArgs) -> Result<()> {
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    for date in NaiveDateRange::new(args.from.succ(), args.to) {
        info!("Recaulculating {}", date);
        db.copy_state(date.pred(), date)?;
        let mut state = db.get_state(date)?;
        let acts = state.get_acts()?;
        // NOTE: this will not handle modifications which modify other
        //       modifications coming into force in the same day
        let modifications = get_all_modifications(&acts, date)?;
        apply_all_modifications(&mut state, &modifications)?;
        state.save()?;
    }
    Ok(())
}
