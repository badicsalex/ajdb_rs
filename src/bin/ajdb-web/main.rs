// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use ajdb::web::web_main;

#[tokio::main]
async fn main() {
    web_main().await
}
