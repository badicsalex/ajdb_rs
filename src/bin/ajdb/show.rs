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

use std::io::stdout;

use ajdb::{database::ActSet, persistence::Persistence};
use anyhow::{bail, Result};
use chrono::{NaiveDate, Utc};
use hun_law::{
    identifier::ActIdentifier,
    output::{CliOutput, OutputFormat},
};

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    #[clap(value_parser, required = true)]
    /// The Act to show in Year/ISSUE format. Example: '2013/31'
    act: ActIdentifier,
    #[clap(value_parser, long, short, default_value_t=Utc::today().naive_utc())]
    /// Get state on the specific date. Format is "2013-12-31". Defaults to today.
    date: NaiveDate,
    /// Output format
    #[clap(value_enum, long, short = 't', default_value_t)]
    output_format: OutputFormat,
    /// Width of the word-wrapped text (applies to text output only)
    #[clap(long, short, default_value = "105")]
    width: usize,
}

pub fn cli_show(args: ShowArgs) -> Result<()> {
    let persistence = Persistence::new("db");
    let state = ActSet::load(&persistence, args.date)?;
    if state.is_empty() {
        bail!("The database is empty at date {}", args.date);
    }
    let act = state.get_act(args.act)?.act()?;
    act.cli_output(args.width, args.output_format, &mut stdout())?;
    Ok(())
}
