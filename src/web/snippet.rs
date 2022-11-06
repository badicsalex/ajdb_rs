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
use chrono::NaiveDate;
use hun_law::{
    identifier::range::{IdentifierRange, IdentifierRangeFrom},
    reference::{parts::AnyReferencePart, Reference},
    util::compact_string::CompactString,
};
use maud::{html, Markup};
use serde::Deserialize;

use super::{
    act::{ConvertToParts, ConvertToPartsContext, DocumentPartMetadata, RenderPartParams},
    util::{link_to_reference, logged_http_error, today, OrToday},
};
use crate::{database::ActSet, persistence::Persistence};

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

    let date = if params.date == Some(today()) {
        None
    } else {
        params.date
    };
    let state = ActSet::load_async(&persistence, date.or_today())
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
        let context = ConvertToPartsContext {
            snippet_range: Some(reference.clone()),
            date,
            part_metadata: DocumentPartMetadata {
                reference: (
                    act_id,
                    IdentifierRange::from_single(article_range.first_in_range()),
                )
                    .into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut parts = Vec::new();
        for article in act
            .articles()
            .filter(|article| article_range.contains(article.identifier))
        {
            if article_range.is_range()
                || matches!(reference.get_last_part(), AnyReferencePart::Article(_))
            {
                article.convert_to_parts(&context, &mut parts)?
            } else {
                article.children.convert_to_parts(&context, &mut parts)?;
            }
        }
        parts
    } else {
        Vec::new()
    };
    let render_part_params = RenderPartParams {
        date,
        convert_links: true,
        force_absolute_urls: true,
        ..Default::default()
    };
    let result = html!(
        @for part in result {
            ( part.render_part(&render_part_params).map_err(logged_http_error)? )
        }
    );
    if let Some(change_cause) = &params.change_cause {
        if change_cause.is_empty() {
            let jat_ref =
                Reference::from_compact_string("2010.130_12_2__").map_err(logged_http_error)?;
            let link = link_to_reference(&jat_ref, Some(date.or_today().succ()), None, true)
                .map_err(logged_http_error)?;
            Ok(html!(
                .modified_by {
                    "Automatikusan hatályát vesztete "
                    ( date.or_today().succ().format("%Y. %m. %d-n").to_string() )
                    " a "
                    ( link )
                    " alapján."
                }
                .previous_state_label {"Korábbi állapot:"}
                .blockamendment_container {
                    (result)
                }
            ))
        } else {
            let cause_ref =
                Reference::from_compact_string(change_cause).map_err(|_| StatusCode::NOT_FOUND)?;
            let link = link_to_reference(&cause_ref, Some(date.or_today().succ()), None, true)
                .map_err(logged_http_error)?;
            if result.0.is_empty() {
                Ok(html!(
                    .modified_by {
                        "Beillesztette "
                        ( date.or_today().succ().format("%Y. %m. %d-n").to_string() )
                        " a "
                        ( link )
                        "."
                    }
                ))
            } else {
                Ok(html!(
                    .modified_by {
                        "Módosíttotta "
                        ( date.or_today().succ().format("%Y. %m. %d-n").to_string() )
                        " a "
                        ( link )
                        "."
                    }
                    .previous_state_label {"Korábbi állapot:"}
                    .blockamendment_container {
                        (result)
                    }
                ))
            }
        }
    } else if result.0.is_empty() {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(result)
    }
}
