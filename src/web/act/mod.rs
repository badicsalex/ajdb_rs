// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.
#[allow(clippy::module_inception)]
mod act;
mod act_children;
mod context;
mod diff;
mod document_part;
mod future_changes;
mod layout;
mod markers;
mod menu;
mod sae;
mod toc;

pub use act::render_act;
use axum::http::StatusCode;
pub use context::ConvertToPartsContext;
pub use diff::{create_diff_pairs, render_act_diff, render_diff_pair};
pub use document_part::{
    DocumentPart, DocumentPartMetadata, DocumentPartSpecific, RenderPartParams,
};

pub trait ConvertToParts {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode>;
}

impl<T: ConvertToParts> ConvertToParts for Vec<T> {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        for child in self {
            child.convert_to_parts(context, output)?
        }
        Ok(())
    }
}
