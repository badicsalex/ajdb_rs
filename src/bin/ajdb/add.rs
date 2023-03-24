// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

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
