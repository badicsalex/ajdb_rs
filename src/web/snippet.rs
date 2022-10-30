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
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use super::{
    act::RenderElement,
    util::{link_to_reference, logged_http_error, RenderElementContext},
};
use crate::{database::ActSet, persistence::Persistence, web::sae::RenderSAE};

#[derive(Debug, Clone, Deserialize)]
pub struct RenderSnippetParams {
    date: Option<NaiveDate>,
    change_cause: Option<String>,
}

pub async fn render_snippet(
    Path(reference_str): Path<String>,
    params: Query<RenderSnippetParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let reference =
        Reference::from_compact_string(reference_str).map_err(|_| StatusCode::NOT_FOUND)?;
    let act_id = reference.act().ok_or(StatusCode::NOT_FOUND)?;

    let today = Utc::today().naive_utc();
    let date = if params.date == Some(today) {
        None
    } else {
        params.date
    };
    let state = ActSet::load_async(&persistence, date.unwrap_or(today))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let act = state
        .get_act(act_id)
        .map_err(|_| StatusCode::NOT_FOUND)?
        .act_cached()
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // This being an `if let` is a huge hack: we generate modification snippet urls for
    // structural elements, which are _not_ supported. But we want to have at least a
    // 'reason' snippet, which is done below if the result here is empty.
    // For non-modification-type snippets, the end result will be a 404
    let result = if let Some(article_range) = reference.article() {
        if article_range.is_range()
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
                                    (act_id, IdentifierRange::from_single(article.identifier))
                                        .into(),
                                ),
                                snippet_range: Some(reference.clone()),
                                date,
                                show_changes: false,
                                force_absolute_urls: true,
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
                current_ref: Some(
                    (act_id, IdentifierRange::from_single(article.identifier)).into(),
                ),
                snippet_range: Some(reference),
                date,
                show_changes: false,
                force_absolute_urls: true,
            })?
        }
    } else {
        PreEscaped(String::new())
    };
    if let Some(change_cause) = &params.change_cause {
        let cause_ref =
            Reference::from_compact_string(change_cause).map_err(|_| StatusCode::NOT_FOUND)?;
        let link =
            link_to_reference(&cause_ref, date, None, true, true).map_err(logged_http_error)?;
        if result.0.is_empty() {
            Ok(html!(
                .modified_by {
                    "Beillesztette "
                    ( date.unwrap_or(today).succ().format("%Y. %m. %d-n").to_string() )
                    " a "
                    ( link )
                    "."
                }
            ))
        } else {
            Ok(html!(
                .modified_by {
                    "Módosíttotta "
                    ( date.unwrap_or(today).succ().format("%Y. %m. %d-n").to_string() )
                    " a "
                    ( link )
                    "."
                }
                .previous_state_label {"Korábbi állapot:"}
                (result)
            ))
        }
    } else if result.0.is_empty() {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(result)
    }
}
