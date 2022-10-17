// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::PathBuf;

use ajdb::{database::ActSet, persistence::Persistence, util::read_all};
use anyhow::{Context, Result};
use hun_law::structure::Act;
use log::info;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    path: PathBuf,
}

pub fn cli_add_raw(args: AddArgs) -> Result<()> {
    let act: Act = hun_law::util::singleton_yaml::from_slice(
        &read_all(&args.path).with_context(|| anyhow::anyhow!("Error reading {:?}", args.path))?,
    )
    .with_context(|| anyhow::anyhow!("Error deserializing {:?}", args.path))?;
    let date = act.publication_date;
    info!("Adding {} to state at {}", act.identifier, date);
    let persistence = Persistence::new("db");
    let mut state = ActSet::load(&persistence, date)?;
    state.store_act(act)?;
    state.save()?;
    Ok(())
}
