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

use std::collections::BTreeMap;

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{
    identifier::IdentifierCommon,
    reference::{to_element::ReferenceToElement, Reference},
    structure::{Act, ChildrenCommon, LastChange, SubArticleElement},
    util::walker::SAEVisitor,
};

#[derive(Debug, Default, Clone)]
pub struct FutureActChanges {
    changes: BTreeMap<Reference, LastChange>,
}

impl FutureActChanges {
    pub fn new(act: &Act, cutoff_date: NaiveDate) -> Result<Self> {
        let mut visitor = ActChangeVisitor {
            cutoff_date,
            result: Default::default(),
        };
        act.walk_saes(&mut visitor)?;
        let mut changes = visitor.result;
        let act_ref = act.reference();
        for article in act.articles() {
            if let Some(last_change) = &article.last_change {
                if last_change.date > cutoff_date {
                    changes.insert(
                        article.reference().relative_to(&act_ref)?,
                        last_change.clone(),
                    );
                }
            }
        }
        Ok(Self { changes })
    }

    pub fn get_change(&self, reference: &Reference) -> Option<&LastChange> {
        self.changes.get(reference)
    }
}

struct ActChangeVisitor {
    cutoff_date: NaiveDate,
    result: BTreeMap<Reference, LastChange>,
}

impl SAEVisitor for ActChangeVisitor {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if let Some(last_change) = &element.last_change {
            if last_change.date > self.cutoff_date {
                self.result.insert(position.clone(), last_change.clone());
            }
        }
        Ok(())
    }
}
