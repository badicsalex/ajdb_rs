// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use ajdb::{
    amender::{AppliableModificationSet, OnError},
    database::{ActMetadata, ActSet},
    persistence::Persistence,
    util::NaiveDateRange,
};
use anyhow::{anyhow, Context, Result};
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
    let persistence = Persistence::new("db");
    for date in NaiveDateRange::new(args.from.succ(), args.to) {
        recalculate_one_date(&persistence, date)
            .with_context(|| anyhow!("Recalculating date {} failed", date))?;
    }
    Ok(())
}

fn recalculate_one_date(persistence: &Persistence, date: NaiveDate) -> Result<()> {
    info!("Recalculating {}", date);
    ActSet::copy(persistence, date.pred(), date)?;
    let mut state = ActSet::load(persistence, date)?;
    let mut act_ids: Vec<_> = state
        .get_acts()?
        .iter()
        .filter(|ae| ae.is_date_interesting(date))
        .map(|ae| ae.identifier())
        .collect();
    if act_ids.is_empty() {
        return Ok(());
    }

    // NOTE: It's important to go in reverse, since there may be later acts
    //       that modify earlier acts on the same enforcement day.
    //       E.g. 2020. évi LXXIV. törvény.yml modifies 2020. évi XLIII. törvény.yml,
    //       both with enforcement dates 2021-01-01, leading to a conflict in Btk.
    act_ids.sort();
    act_ids.reverse();

    let mut modifications = AppliableModificationSet::default();
    modifications.add_fixups(date)?;
    for act_id in &act_ids {
        // NOTE: And then there's the case where an Act is modified by one Act, and then another,
        //       Both coming into force at the same time. This is resolved by the internal
        //       ordering fix in modifications.apply_to_act(...)
        modifications.apply_to_act_in_state(*act_id, date, &mut state, OnError::Warn)?;
        modifications.remove_affecting(*act_id);
        let act = state.get_act(*act_id)?.act()?;
        modifications.add(&act, date)?;
    }

    let mut modified_acts = act_ids; //no clone necessary
    modified_acts.append(&mut modifications.affected_acts());
    for act_id in modified_acts {
        if state.has_act(act_id) {
            let mut act_metadata = ActMetadata::load(persistence, act_id)?;
            act_metadata.add_modification_date(date)?;
            act_metadata.save()?;
        }
    }

    modifications.apply_rest(date, &mut state, OnError::Warn)?;
    state.save()?;
    Ok(())
}
