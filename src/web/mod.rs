// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod act;
mod index;
mod snippet;
mod util;

use std::{net::SocketAddr, sync::Arc};

use self::{
    act::{render_act, render_act_diff},
    index::render_index,
    snippet::{render_diff_snippet, render_snippet},
};
use crate::persistence::Persistence;

pub async fn web_main() {
    let persistence = Persistence::new("db");
    let router = axum::Router::new()
        .route("/", axum::routing::get(render_index))
        .route("/act/:act_id", axum::routing::get(render_act))
        .route("/diff/:act_id", axum::routing::get(render_act_diff))
        .route("/snippet/:snippet_ref", axum::routing::get(render_snippet))
        .route(
            "/diff_snippet/:snippet_ref",
            axum::routing::get(render_diff_snippet),
        )
        .merge(axum_extra::routing::SpaRouter::new(
            "/static",
            "src/web/static",
        ))
        .layer(axum::extract::Extension(Arc::new(persistence)));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
