// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

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
