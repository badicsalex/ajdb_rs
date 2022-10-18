// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::io::Write;

use ajdb::web::web_main;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    )
    .format(|buf, record| writeln!(buf, "{:>5}: {}", record.level(), record.args()))
    .init();

    web_main().await
}
