// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{iter::repeat, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use chrono::NaiveDate;
use hun_law::structure::Act;
use maud::{html, Markup};
use serde::Deserialize;
use similar::capture_diff_slices;

use super::{
    act::convert_act_to_parts,
    document_part::{DocumentPartSpecific, RenderPartParams},
    layout::document_layout,
    menu::render_act_menu,
    toc::generate_toc,
    DocumentPart,
};
use crate::{
    database::{ActMetadata, ActSet},
    persistence::Persistence,
    web::util::{anchor_string, article_anchor, logged_http_error, OrToday},
};

#[derive(Debug, Clone, Deserialize)]
pub struct RenderActDiffParams {
    date_left: Option<NaiveDate>,
    date_right: Option<NaiveDate>,
}

pub async fn render_act_diff<'a>(
    Path(act_id_str): Path<String>,
    params: Query<RenderActDiffParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let diff_data = get_act_diff_data(&act_id_str, &params, &persistence)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(document_layout(
        "act_diff",
        diff_data.act_left.identifier.to_string(),
        generate_toc(&diff_data.act_left),
        render_act_menu(
            diff_data.act_left.identifier,
            diff_data.date_left,
            diff_data.act_left.publication_date,
            diff_data.modification_dates.clone(),
        ),
        render_act_diff_body(&diff_data)?,
    ))
}

struct ActDiffData {
    act_left: Arc<Act>,
    date_left: NaiveDate,
    act_right: Arc<Act>,
    date_right: NaiveDate,
    modification_dates: Vec<NaiveDate>,
}

async fn get_act_diff_data(
    act_id_str: &str,
    params: &RenderActDiffParams,
    persistence: &Persistence,
) -> anyhow::Result<ActDiffData> {
    let act_id = act_id_str.parse()?;

    let date_right = params.date_right.or_today();
    let state_right = ActSet::load_async(persistence, date_right).await?;
    let act_right = state_right.get_act(act_id)?.act_cached().await?;

    let date_left = params.date_left.unwrap_or(act_right.publication_date);
    let state_left = ActSet::load_async(persistence, date_left).await?;
    let act_left = state_left.get_act(act_id)?.act_cached().await?;

    let act_metadata = ActMetadata::load_async(persistence, act_id).await?;
    let modification_dates = act_metadata.modification_dates();
    Ok(ActDiffData {
        act_left,
        date_left,
        act_right,
        date_right,
        modification_dates,
    })
}

fn render_act_diff_body(diff_data: &ActDiffData) -> Result<Markup, StatusCode> {
    let body_parts_left = convert_act_to_parts(&diff_data.act_left, diff_data.date_left)?;
    let body_parts_right = convert_act_to_parts(&diff_data.act_right, diff_data.date_right)?;

    let render_params_left = RenderPartParams {
        date: Some(diff_data.date_left),
        element_anchors: true,
        convert_links: true,
        ..Default::default()
    };
    let render_params_right = RenderPartParams {
        date: Some(diff_data.date_right),
        convert_links: true,
        ..Default::default()
    };

    Ok(html!(
        .act_title {
            (diff_data.act_left.identifier.to_string())
            br;
            (diff_data.act_left.subject)
        }
        @for (left, right) in create_diff_pairs(&body_parts_left, &body_parts_right) {
            ( render_diff_pair(left, right, &render_params_left, &render_params_right)? )
        }
    ))
}

fn create_diff_pairs<'a, 'b>(
    left: &'a [DocumentPart<'b>],
    right: &'a [DocumentPart<'b>],
) -> Vec<(Option<&'a DocumentPart<'b>>, Option<&'a DocumentPart<'b>>)> {
    let diffme_left: Vec<_> = left.iter().map(part_to_diffable_string).collect();
    let diffme_right: Vec<_> = right.iter().map(part_to_diffable_string).collect();
    let mut result = Vec::new();
    for diff_op in capture_diff_slices(similar::Algorithm::Patience, &diffme_left, &diffme_right) {
        match diff_op {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => result.extend(
                left.iter()
                    .skip(old_index)
                    .map(Some)
                    .zip(right.iter().skip(new_index).map(Some))
                    .take(len),
            ),
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => result.extend(
                left.iter()
                    .skip(old_index)
                    .map(|p| (Some(p), None))
                    .take(old_len),
            ),
            similar::DiffOp::Insert {
                new_index, new_len, ..
            } => result.extend(
                right
                    .iter()
                    .skip(new_index)
                    .map(|p| (None, Some(p)))
                    .take(new_len),
            ),
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let iter_left = left
                    .iter()
                    .skip(old_index)
                    .map(Some)
                    .take(old_len)
                    .chain(repeat(None));
                let iter_right = right
                    .iter()
                    .skip(new_index)
                    .map(Some)
                    .take(new_len)
                    .chain(repeat(None));
                result.extend(iter_left.zip(iter_right).take(old_len.max(new_len)));
            }
        }
    }
    result
}

fn part_to_diffable_string(part: &DocumentPart) -> String {
    match &part.specifics {
        DocumentPartSpecific::StructuralElement { id, .. } => id.clone(),
        DocumentPartSpecific::SAEText(text_part) => {
            if text_part.show_article_header {
                article_anchor(&part.metadata.reference)
            } else {
                text_part.text.to_string()
            }
        }
        _ => anchor_string(&part.metadata.reference),
    }
}

fn render_diff_pair(
    left: Option<&DocumentPart>,
    right: Option<&DocumentPart>,
    left_params: &RenderPartParams,
    right_params: &RenderPartParams,
) -> Result<Markup, StatusCode> {
    let different = match (left, right) {
        (None, None) => false, // Should not happen
        (None, Some(_)) => true,
        (Some(_), None) => true,
        (Some(l), Some(r)) => match (&l.specifics, &r.specifics) {
            (DocumentPartSpecific::SAEText(part_l), DocumentPartSpecific::SAEText(part_r)) => {
                part_l.text != part_r.text
            }
            (ls, rs) => ls != rs,
        },
    };

    Ok(html!(
        .diff_container {
            .diff_left .different[different && left.is_some()]{
                @if let Some(left) = left {
                    (left.render_part(left_params).map_err(logged_http_error)?)
                }
            }
            .diff_right .different[different && right.is_some()]{
                @if let Some(right) = right {
                    (right.render_part(right_params).map_err(logged_http_error)?)
                }
            }
        }
    ))
}
