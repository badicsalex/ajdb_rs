// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use maud::{html, Markup, DOCTYPE};

use super::util::logged_http_error;
use crate::{database::ActSet, persistence::Persistence};

fn get_all_acts() -> Result<Vec<String>> {
    let persistence = Persistence::new("db");
    let state = ActSet::load(&persistence, "2022-09-30".parse()?)?;
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
                    .title {
                        "Alex Jogi Adatbázisa"
                    }
                    .inner_container {
                        p {
                            "Egy fejlesztés alatt lévő, a "
                            a href="https://github.com/badicsalex/hun_law_rs" { "hun_law keretrendszerre"}
                            " épülő jogtár (és egyéb) projekt."
                            br;
                            "Használata csak saját felelősségre (kezelje úgy, mintha az itt lévő adatok 100%-a hibás lenne)."
                            br;
                            br;
                            "Ha kérdése, észrevétele, ötlete van, midnenképpen küldjön egy levelet a"
                            br;
                            a href="mailto:info@ajdb.hu" { "info@ajdb.hu" }
                            " vagy "
                            a href="mailto:admin@stickman.hu" { "admin@stickman.hu" }
                            " címre."
                        }
                        h3 { "Fontos elérhető törvények:" }
                        a href="/act/2012-100" .important_act {
                            "Btk."
                        }
                        a href="/act/2012-1" .important_act {
                            "Mt."
                        }
                        a href="/act/2013-5" .important_act {
                            "Ptk."
                        }
                        h3 { "Egyéb törvények:" }
                        ul {
                            @for act in acts {
                                li { a href={"/act/" (act)} { (act) } }
                            }
                        }
                    }
                }
            }
        }
    ))
}
