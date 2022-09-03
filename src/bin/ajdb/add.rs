// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::PathBuf;

use anyhow::Result;

use ajdb::{database::Database, persistence::Persistence, util::read_all};
use hun_law::structure::Act;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    path: PathBuf,
}

pub fn cli_add_raw(args: AddArgs) -> Result<()> {
    let act: Act = hun_law::util::singleton_yaml::from_slice(&read_all(args.path)?)?;
    let date = act.publication_date;
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    let mut state = db.get_state(date)?;
    state.store_act(act)?;
    state.save()?;
    Ok(())
}
