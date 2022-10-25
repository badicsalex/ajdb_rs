// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod act;
mod act_toc;
mod index;
mod sae;
mod util;

use std::{net::SocketAddr, sync::Arc};

use crate::persistence::Persistence;

use self::{act::render_act, index::render_index};

pub async fn web_main() {
    let persistence = Persistence::new("db");
    let router = axum::Router::new()
        .route("/", axum::routing::get(render_index))
        .route("/act/:act_id", axum::routing::get(render_act))
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
