// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::{Path, PathBuf};

use ajdb::{database::ActSet, persistence::Persistence, util::read_all};
use anyhow::{anyhow, Context, Result};
use hun_law::structure::Act;
use log::info;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    #[clap(required = true, name = "path")]
    paths: Vec<PathBuf>,
}

pub fn cli_add_raw(args: AddArgs) -> Result<()> {
    let mut everything_ok = true;
    for path in &args.paths {
        if let Err(err) = add_path(path) {
            log::error!("{err:?}");
            everything_ok = false;
        }
    }
    if everything_ok {
        Ok(())
    } else {
        Err(anyhow!("Some acts were not processed"))
    }
}

fn add_path(path: &Path) -> Result<()> {
    let act: Act = hun_law::util::singleton_yaml::from_slice(
        &read_all(path).with_context(|| anyhow!("Error reading {path:?}"))?,
    )
    .with_context(|| anyhow!("Error deserializing {path:?}"))?;
    let date = act.publication_date;
    info!("Adding {} to state at {date}", act.identifier);
    let persistence = Persistence::new("db");
    let mut state = ActSet::load(&persistence, date)?;
    state.store_act(act)?;
    state.save()?;
    Ok(())
}
