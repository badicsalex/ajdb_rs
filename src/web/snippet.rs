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
    identifier::ActIdentifier,
    reference::{parts::AnyReferencePart, Reference},
    structure::{Act, ChangeCause},
    util::compact_string::CompactString,
};
use maud::{html, Markup};
use serde::Deserialize;

use super::{
    act::{
        ConvertToParts, ConvertToPartsContext, DocumentPart, DocumentPartMetadata,
        DocumentPartSpecific, RenderPartParams,
    },
    util::{logged_http_error, modified_by_text, today, OrToday},
};
use crate::{
    database::ActSet,
    persistence::Persistence,
    web::act::{create_diff_pairs, render_diff_pair},
};

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

    let date = params.date.or_today();
    let act = get_act(&persistence, act_id, date)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let parts = get_snippet_as_document_parts(&act, &reference, date)?;

    let render_part_params = RenderPartParams {
        date: if date == today() { None } else { Some(date) },
        convert_links: true,
        force_absolute_urls: true,
        ..Default::default()
    };
    let result = html!(
        .act_snippet {
            @for part in parts {
                ( part.render_part(&render_part_params).map_err(logged_http_error)? )
            }
        }
    );
    if result.0.is_empty() {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(result)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderDiffSnippetParams {
    date_left: NaiveDate,
    date_right: NaiveDate,
    change_cause: String,
}

pub async fn render_diff_snippet(
    Path(reference_str): Path<String>,
    params: Query<RenderDiffSnippetParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let reference =
        Reference::from_compact_string(reference_str).map_err(|_| StatusCode::NOT_FOUND)?;
    let act_id = reference.act().ok_or(StatusCode::NOT_FOUND)?;

    let act_left = get_act(&persistence, act_id, params.date_left)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let act_right = get_act(&persistence, act_id, params.date_right)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let parts_left = get_snippet_as_document_parts(&act_left, &reference, params.date_left)?;
    let parts_right = get_snippet_as_document_parts(&act_right, &reference, params.date_right)?;

    let verb = match (
        only_empty_parts(&parts_left),
        only_empty_parts(&parts_right),
    ) {
        (true, true) => "Módosítva", // ???? Should not happen
        (true, false) => "Beillesztve",
        (false, true) => "Hatályon kívül helyezve",
        (false, false) => "Módosítva",
    };
    let modified_by = modified_by_text(
        params.date_left.succ(),
        &if params.change_cause.is_empty() {
            ChangeCause::AutoRepeal
        } else if params.change_cause.starts_with("other:") {
            ChangeCause::Other(params.change_cause[6..].to_string())
        } else {
            ChangeCause::Amendment(
                Reference::from_compact_string(&params.change_cause)
                    .map_err(|_| StatusCode::NOT_FOUND)?,
            )
        },
        verb,
    )?;
    let render_params_left = RenderPartParams {
        date: Some(params.date_left),
        render_past_change_marker: true,
        convert_links: true,
        force_absolute_urls: true,
        ..Default::default()
    };
    let render_params_right = RenderPartParams {
        date: Some(params.date_right),
        convert_links: true,
        force_absolute_urls: true,
        ..Default::default()
    };
    if only_empty_parts(&parts_left) {
        Ok(html!(
            .act_snippet {
                .modified_by { ( modified_by ) }
                @for part in parts_right {
                    .diff_right .different .diff_full {
                        (part.render_part(&render_params_right).map_err(logged_http_error)?)
                    }
                }
            }
        ))
    } else if only_empty_parts(&parts_right) {
        Ok(html!(
            .act_snippet {
                .modified_by { ( modified_by ) }
                @for part in parts_left {
                    .diff_left .different .diff_full {
                        (part.render_part(&render_params_left).map_err(logged_http_error)?)
                    }
                }
            }
        ))
    } else {
        Ok(html!(
            .diff_snippet {
                .modified_by { ( modified_by ) }
                @for (left, right) in create_diff_pairs(&parts_left, &parts_right) {
                    ( render_diff_pair(left, &render_params_left, right, &render_params_right)? )
                }
            }
        ))
    }
}

async fn get_act(
    persistence: &Persistence,
    act_id: ActIdentifier,
    date: NaiveDate,
) -> anyhow::Result<Arc<Act>> {
    let state = ActSet::load_async(persistence, date).await?;
    state.get_act(act_id)?.act_cached().await
}

fn get_snippet_as_document_parts<'a>(
    act: &'a Act,
    reference: &Reference,
    date: NaiveDate,
) -> Result<Vec<DocumentPart<'a>>, StatusCode> {
    let act_id = reference.act().ok_or(StatusCode::NOT_FOUND)?;
    let article_range = reference.article().ok_or(StatusCode::NOT_FOUND)?;

    let context = ConvertToPartsContext {
        snippet_range: Some(reference.clone()),
        date,
        part_metadata: DocumentPartMetadata {
            reference: act_id.into(),
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
            let context = context
                .clone()
                .relative_to(article)?
                .update_change_markers(article.last_change.as_ref());
            article.children.convert_to_parts(&context, &mut parts)?;
        }
    }

    Ok(parts)
}

fn only_empty_parts(parts: &[DocumentPart]) -> bool {
    for part in parts {
        if let DocumentPartSpecific::SAEText(sae) = &part.specifics {
            if !sae.text.is_empty() {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}
