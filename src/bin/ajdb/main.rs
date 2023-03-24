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
