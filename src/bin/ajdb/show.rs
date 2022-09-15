// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::io::stdout;

use ajdb::{database::Database, persistence::Persistence};
use anyhow::{Result, bail};
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
}

pub fn cli_show(args: ShowArgs) -> Result<()> {
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    let state = db.get_state(args.date)?;
    if state.is_empty() {
        bail!("The database is empty at date {}", args.date);
    }
    let act = state.get_act(args.act)?.act()?;
    act.cli_output(args.output_format, &mut stdout())?;
    Ok(())
}
