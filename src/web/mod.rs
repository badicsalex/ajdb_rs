// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod act;
mod index;

use std::net::SocketAddr;

use self::{act::render_act, index::render_index};

pub async fn web_main() {
    let router = axum::Router::new()
        .route("/", axum::routing::get(render_index))
        .route("/act/:act_id", axum::routing::get(render_act))
        .merge(axum_extra::routing::SpaRouter::new(
            "/static",
            "src/web/static",
        ));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
