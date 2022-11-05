// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.
#[allow(clippy::module_inception)]
pub mod act;
pub mod act_children;
pub mod context;
pub mod layout;
pub mod markers;
pub mod menu;
pub mod sae;
pub mod toc;

use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use chrono::NaiveDate;
use hun_law::identifier::ActIdentifier;
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use self::{
    context::RenderElementContext, layout::document_layout, menu::render_act_menu,
    toc::generate_toc,
};
use super::util::{today, OrToday};
use crate::{
    database::{ActMetadata, ActSet},
    persistence::Persistence,
};

pub trait RenderElement {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderActParams {
    date: Option<NaiveDate>,
}

pub async fn render_existing_act<'a>(
    act_id: ActIdentifier,
    date: NaiveDate,
    state: &'a ActSet<'a>,
    persistence: &'a Persistence,
) -> Result<Markup, StatusCode> {
    let act = state
        .get_act(act_id)
        .map_err(|_| StatusCode::NOT_FOUND)?
        .act_cached()
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let act_metadata = ActMetadata::load_async(persistence, act_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let modification_dates = act_metadata.modification_dates();
    let act_render_context = RenderElementContext {
        date: if date == today() { None } else { Some(date) },
        show_changes: true,
        ..Default::default()
    };
    Ok(document_layout(
        act.identifier.to_string(),
        generate_toc(&act),
        render_act_menu(
            act.identifier,
            date,
            act.publication_date,
            modification_dates,
        ),
        act.render(&act_render_context)?,
    ))
}

pub fn render_nonexistent_act(act_id: ActIdentifier) -> Result<Markup, StatusCode> {
    let njt_link = format!(
        "https://njt.hu/jogszabaly/{}-{}-00-00",
        act_id.year, act_id.number
    );
    Ok(document_layout(
        act_id.to_string(),
        PreEscaped(String::new()),
        html!(
            .menu_act_title { ( act_id.to_string() ) }
        ),
        html!(
            .not_found {
                "A " ( act_id.to_string() ) " még nincs felvéve az adatbázisba."
                br;
                br;
                a href=(njt_link) { "Ezen a linken" }
                " elérheti a Nemzeti Jogtáron található verziót"
            }
        ),
    ))
}

pub async fn render_act(
    Path(act_id_str): Path<String>,
    params: Query<RenderActParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let act_id = act_id_str.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let date = params.date.or_today();
    let state = ActSet::load_async(&persistence, date)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if state.has_act(act_id) {
        render_existing_act(act_id, date, &state, &persistence).await
    } else {
        render_nonexistent_act(act_id)
    }
}
