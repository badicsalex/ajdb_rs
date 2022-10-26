// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use chrono::{NaiveDate, Utc};
use hun_law::{reference::Reference, util::compact_string::CompactString};
use maud::Markup;
use serde::Deserialize;

use super::util::RenderElementContext;
use crate::{database::ActSet, persistence::Persistence, web::sae::RenderSAE};

#[derive(Debug, Clone, Deserialize)]
pub struct RenderSnippetParams {
    date: Option<NaiveDate>,
}

pub async fn render_snippet(
    Path(reference_str): Path<String>,
    params: Query<RenderSnippetParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let reference =
        Reference::from_compact_string(reference_str).map_err(|_| StatusCode::NOT_FOUND)?;
    let act_id = reference.act().ok_or(StatusCode::NOT_FOUND)?;
    let article_range = reference.article().ok_or(StatusCode::NOT_FOUND)?;

    let today = Utc::today().naive_utc();
    let date = params.date.unwrap_or(today);
    let state = ActSet::load_async(&*persistence, date)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let act = state
        .get_act(act_id)
        .map_err(|_| StatusCode::NOT_FOUND)?
        .act_cached()
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let article = act
        .article(article_range.first_in_range())
        .ok_or(StatusCode::NOT_FOUND)?;

    article.children.render(&RenderElementContext {
        current_ref: Some((act_id, article_range).into()),
        snippet_range: Some(reference),
        date: if date == today { None } else { Some(date) },
    })
}
