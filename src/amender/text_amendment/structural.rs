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

use anyhow::{bail, ensure, Result};
use hun_law::{
    reference::structural::StructuralReference,
    structure::{Act, ActChild, LastChange},
};

use super::NeedsFullReparse;
use crate::{
    amender::text_amendment::text_replace::normalized_replace, structural_cut_points::GetCutPoints,
};

pub fn apply_structural_title_amendment(
    reference: &StructuralReference,
    from: &str,
    to: &str,
    act: &mut Act,
    change_entry: &LastChange,
) -> Result<NeedsFullReparse> {
    let mut applied = false;
    let cut_points = reference.get_cut_points(act, false)?;
    match &mut act.children[cut_points.start] {
        ActChild::StructuralElement(se) => {
            if let Some(replaced) = normalized_replace(&se.title, from, to) {
                se.title = replaced;
                applied = true;
                se.last_change = Some(change_entry.clone());
            }
        }
        ActChild::Subtitle(st) => {
            if let Some(replaced) = normalized_replace(&st.title, from, to) {
                st.title = replaced;
                applied = true;
                st.last_change = Some(change_entry.clone());
            }
        }
        ActChild::Article(_) => {
            bail!("Computed target of a structural title amendment ({reference:?}) was an article.")
        }
    }
    ensure!(
        applied,
        "Article title amendment @{reference:?} from={from:?} to={to:?} did not have an effect",
    );
    Ok(NeedsFullReparse::No)
}
