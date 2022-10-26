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
use hun_law::{
    identifier::range::{IdentifierRange, IdentifierRangeFrom},
    reference::{parts::AnyReferencePart, Reference},
    util::compact_string::CompactString,
};
use maud::{Markup, PreEscaped};
use serde::Deserialize;

use super::{act::RenderElement, util::RenderElementContext};
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

    let result = if article_range.is_range()
        || matches!(reference.get_last_part(), AnyReferencePart::Article(_))
    {
        let rendered_articles = act
            .articles()
            .filter(|article| article_range.contains(article.identifier))
            .map(|article| {
                article
                    .render(
                        &RenderElementContext {
                            current_ref: Some(
                                (act_id, IdentifierRange::from_single(article.identifier)).into(),
                            ),
                            snippet_range: Some(reference.clone()),
                            date: if date == today { None } else { Some(date) },
                        },
                        None,
                    )
                    .map(|r| r.0)
            })
            .collect::<Result<String, StatusCode>>()?;
        PreEscaped(rendered_articles)
    } else {
        let article = act
            .article(article_range.first_in_range())
            .ok_or(StatusCode::NOT_FOUND)?;

        article.children.render(&RenderElementContext {
            current_ref: Some((act_id, IdentifierRange::from_single(article.identifier)).into()),
            snippet_range: Some(reference),
            date: if date == today { None } else { Some(date) },
        })?
    };
    if result.0.is_empty() {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(result)
    }
}
