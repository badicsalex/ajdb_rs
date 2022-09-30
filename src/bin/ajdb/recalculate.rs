// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use ajdb::{
    amender::AppliableModificationSet, database::Database, persistence::Persistence,
    util::NaiveDateRange,
};
use anyhow::{anyhow, bail, Context, Result};
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
        recalculate_one_date(&mut db, date)
            .with_context(|| anyhow!("Recalculating date {} failed", date))?;
    }
    Ok(())
}

fn recalculate_one_date(db: &mut Database, date: NaiveDate) -> Result<()> {
    info!("Recalculating {}", date);
    db.copy_state(date.pred(), date)?;
    let mut state = db.get_state(date)?;
    let mut act_ids: Vec<_> = state
        .get_acts()?
        .iter()
        .filter(|ae| ae.is_date_interesting(date))
        .map(|ae| ae.identifier())
        .collect();

    // NOTE: It's important to go in reverse, since there may be later acts
    //       that modify earlier acts on the same enforcement day.
    //       E.g. 2020. évi LXXIV. törvény.yml modifies 2020. évi XLIII. törvény.yml,
    //       both with enforcement dates 2021-01-01, leading to a conflict in Btk.
    act_ids.sort();
    act_ids.reverse();

    let mut applied_acts = Vec::new();
    for act_id in &act_ids {
        let act = state.get_act(*act_id)?.act()?;
        let modifications = AppliableModificationSet::from_acts(&[act], date)?;
        for applied_act in &applied_acts {
            if modifications.affects(*applied_act) {
                bail!(
                        "There is a modification in {} that affects an already applied act {} on date {}",
                        act_id,
                        applied_act,
                        date,
                    );
            }
        }
        modifications.apply(&mut state)?;
        applied_acts.push(*act_id);
    }
    state.save()?;
    Ok(())
}
