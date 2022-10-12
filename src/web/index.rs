// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use maud::{html, Markup, DOCTYPE};

use crate::{database::Database, persistence::Persistence};

use super::util::logged_http_error;

fn get_all_acts() -> Result<Vec<String>> {
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    let state = db.get_state("2022-09-30".parse()?)?;
    let acts = state.get_acts()?;
    Ok(acts
        .into_iter()
        .map(|ae| ae.identifier().to_string())
        .collect())
}

pub async fn render_index() -> Result<Markup, StatusCode> {
    let acts = get_all_acts().map_err(logged_http_error)?;

    Ok(html!(
        (DOCTYPE)
        html {
            head {
                title { "AJDB" }
                link rel="stylesheet" href="/static/style_common.css";
                link rel="stylesheet" href="/static/style_portal.css";
                link rel="icon" href="/static/favicon.png";
            }
            body {
                .main_container {
                    h1 { "Welcome to AJDB" }
                    h3 { "We have the following acts:" }
                    @for act in acts {
                        li { a href={"/act/" (act)} { (act) } }
                    }
                }
            }
        }
    ))
}
