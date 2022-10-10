// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Error;
use axum::http::StatusCode;

pub fn logged_http_error(e: Error) -> StatusCode {
    log::error!("Internal error occured: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}
