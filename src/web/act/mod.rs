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
