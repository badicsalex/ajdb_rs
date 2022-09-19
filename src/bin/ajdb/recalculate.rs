// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use ajdb::{
    amender::AppliableModificationSet, database::Database, persistence::Persistence,
    util::NaiveDateRange,
};
use anyhow::Result;
use chrono::NaiveDate;
use hun_law::structure::Act;
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
        info!("Recalculating {}", date);
        db.copy_state(date.pred(), date)?;
        let mut state = db.get_state(date)?;
        let acts = state
            .get_acts()?
            .iter()
            .map(|ae| ae.act())
            .collect::<Result<Vec<Act>>>()?;
        // NOTE: this will not handle modifications which modify other
        //       modifications coming into force in the same day
        let modifications = AppliableModificationSet::from_acts(acts.iter(), date)?;
        modifications.apply(&mut state)?;
        state.save()?;
    }
    Ok(())
}
