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

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    reference::structural::{StructuralReference, StructuralReferenceElement},
    structure::{Act, ActChild, Article, LastChange},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};
use crate::structural_cut_points::GetCutPoints;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralBlockAmendmentWithContent {
    pub position: StructuralReference,
    pub pure_insertion: bool,
    pub content: Vec<ActChild>,
}

impl ModifyAct for StructuralBlockAmendmentWithContent {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        let cut = self.position.get_cut_points(act, self.pure_insertion)?;
        let mut tail = act.children.split_off(cut.end);
        if self.content.is_empty() {
            let cut_out = act.children.split_off(cut.start);
            act.children.extend(cut_out.into_iter().filter_map(|c| {
                if let ActChild::Article(a) = c {
                    Some(ActChild::Article(Article {
                        identifier: a.identifier,
                        title: None,
                        children: Vec::new(),
                        last_change: Some(change_entry.clone()),
                    }))
                } else {
                    None
                }
            }));
        } else {
            act.children.truncate(cut.start);
            let content = self.content.iter().map(|c| {
                let mut result = c.clone();
                match &mut result {
                    ActChild::StructuralElement(x) => x.last_change = Some(change_entry.clone()),
                    ActChild::Subtitle(x) => x.last_change = Some(change_entry.clone()),
                    ActChild::Article(x) => x.last_change = Some(change_entry.clone()),
                }
                result
            });
            act.children.extend(content);
        }
        act.children.append(&mut tail);
        if let StructuralReferenceElement::Article(article_ids) = self.position.structural_element {
            if !article_ids.is_range() {
                let abbrevs_changed =
                    act.add_semantic_info_to_article(article_ids.first_in_range())?;
                return Ok(abbrevs_changed.into());
            }
        }
        Ok(NeedsFullReparse::Yes)
    }
}

impl AffectedAct for StructuralBlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act.ok_or_else(|| {
            anyhow!("No act in reference in special phrase (StructuralBlockAmendmentWithContent))")
        })
    }
}
