// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod add;
mod recalculate;
mod show;

use std::io::Write;

use add::{cli_add_raw, AddArgs};
use anyhow::Result;
use clap::Parser;
use recalculate::{cli_recalculate, RecalculateArgs};
use show::{cli_show, ShowArgs};

/// AJDB command line interface
///
/// Manages the DB itself with various subcommands
#[derive(clap::Parser, Debug)]
struct AjdbArgs {
    #[clap(subcommand)]
    command: AjdbCommand,
}

#[derive(clap::Subcommand, Debug)]
enum AjdbCommand {
    /// Add raw acts as parsed from MK. Usually created by the default invocation of hun_law
    Add(AddArgs),
    /// Recalculate amendments in the given date range. Be sure that the end date range is the end
    /// of the actual database, or else the db will be inconsistent.
    Recalculate(RecalculateArgs),
    /// Show a single act at a specific date
    Show(ShowArgs),
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    )
    .format(|buf, record| writeln!(buf, "{:>5}: {}", record.level(), record.args()))
    .init();

    let args = AjdbArgs::parse();
    match args.command {
        AjdbCommand::Add(a) => cli_add_raw(a),
        AjdbCommand::Recalculate(a) => cli_recalculate(a),
        AjdbCommand::Show(a) => cli_show(a),
    }
}
