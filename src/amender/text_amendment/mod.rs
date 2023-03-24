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

mod article_title;
mod sae;
mod structural;
mod text_replace;

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    semantic_info::{TextAmendment, TextAmendmentReference},
    structure::{Act, LastChange},
};

use self::{
    article_title::apply_article_title_amendment, sae::apply_sae_text_amendment,
    structural::apply_structural_title_amendment,
};
use super::{AffectedAct, ModifyAct, NeedsFullReparse};

impl ModifyAct for TextAmendment {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        match &self.reference {
            TextAmendmentReference::SAE {
                reference,
                amended_part,
            } => apply_sae_text_amendment(
                reference,
                amended_part,
                &self.from,
                &self.to,
                act,
                change_entry,
            ),
            TextAmendmentReference::Structural(reference) => {
                apply_structural_title_amendment(reference, &self.from, &self.to, act, change_entry)
            }
            TextAmendmentReference::ArticleTitle(reference) => {
                apply_article_title_amendment(reference, &self.from, &self.to, act, change_entry)
            }
        }
    }
}

impl AffectedAct for TextAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        match &self.reference {
            TextAmendmentReference::SAE { reference, .. } => reference.act(),
            TextAmendmentReference::Structural(reference) => reference.act,
            TextAmendmentReference::ArticleTitle(reference) => reference.act(),
        }
        .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))
    }
}
