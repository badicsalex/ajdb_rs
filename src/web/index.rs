// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::sync::Arc;

use anyhow::Result;
use axum::{http::StatusCode, Extension};
use maud::{html, Markup, DOCTYPE};

use super::util::logged_http_error;
use crate::{database::ActSet, persistence::Persistence};

async fn get_all_acts(persistence: &Persistence) -> Result<Vec<String>> {
    let state = ActSet::load_async(persistence, "2022-09-30".parse()?).await?;
    let acts = state.get_acts()?;
    Ok(acts
        .into_iter()
        .map(|ae| ae.identifier().to_string())
        .collect())
}

pub async fn render_index(
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let acts = get_all_acts(&persistence)
        .await
        .map_err(logged_http_error)?;

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
                            "Ha kérdése, észrevétele, ötlete van, mindenképpen küldjön egy levelet az "
                            a style="font-weight: bold" href="mailto:info@ajdb.hu" { "info@ajdb.hu" }
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
