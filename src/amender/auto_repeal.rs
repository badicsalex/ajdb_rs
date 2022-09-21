// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{
    identifier::IdentifierCommon, reference::Reference, semantic_info::SpecialPhrase,
    structure::SubArticleElement, util::walker::SAEVisitor,
};

use crate::enforcement_date_set::EnforcementDateSet;

use super::{repeal::SimplifiedRepeal, AppliableModification};

/// Auto-repeal of modifications according to
/// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
/// Also parses inline repeals
#[derive(Debug)]
pub struct AutoRepealAccumulator<'a> {
    ed_set: &'a EnforcementDateSet,
    date: NaiveDate,
    positions: Vec<Reference>,
}

impl<'a> SAEVisitor for AutoRepealAccumulator<'a> {
    fn on_enter<IT: IdentifierCommon, CT>(
        &mut self,
        position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if let Some(phrase) = &element.semantic_info.special_phrase {
            match phrase {
                SpecialPhrase::ArticleTitleAmendment(_)
                | SpecialPhrase::BlockAmendment(_)
                | SpecialPhrase::Repeal(_)
                | SpecialPhrase::TextAmendment(_)
                | SpecialPhrase::StructuralBlockAmendment(_)
                | SpecialPhrase::StructuralRepeal(_) => {
                    if self.ed_set.came_into_force_yesterday(position, self.date)? {
                        self.positions.push(position.clone())
                    }
                }
                // Special handling for inline repeal
                SpecialPhrase::EnforcementDate(ed) => {
                    if ed.inline_repeal.map_or(false, |d| d == self.date) {
                        self.positions.push(Reference::default());
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a> AutoRepealAccumulator<'a> {
    pub fn new(ed_set: &'a EnforcementDateSet, date: NaiveDate) -> Self {
        Self {
            ed_set,
            date,
            positions: Vec::new(),
        }
    }

    pub fn get_result(self, act_ref: &Reference) -> Result<Vec<AppliableModification>> {
        self.positions
            .into_iter()
            .map(|p| {
                Ok(AppliableModification {
                    modification: SimplifiedRepeal {
                        position: p.relative_to(act_ref)?,
                    }
                    .into(),
                    source: None,
                })
            })
            .collect::<Result<Vec<_>>>()
    }
}
