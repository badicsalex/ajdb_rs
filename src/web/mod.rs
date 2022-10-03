// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::net::SocketAddr;

use self::index::render_index;

mod index;

pub async fn web_main() {
    let router = axum::Router::new()
        .route("/", axum::routing::get(render_index))
        .merge(axum_extra::routing::SpaRouter::new("/static", "static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
