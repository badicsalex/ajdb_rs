// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{
    identifier::IdentifierCommon,
    reference::Reference,
    semantic_info::SpecialPhrase,
    structure::{ChildrenCommon, SubArticleElement},
    util::walker::SAEVisitor,
};

use crate::enforcement_date_set::EnforcementDateSet;

use super::{repeal::SimplifiedRepeal, AppliableModification};

/// Auto-repeal of modifications according to
/// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
#[derive(Debug)]
pub struct AutoRepealAccumulator<'a> {
    ed_set: &'a EnforcementDateSet,
    date: NaiveDate,
    fixups: &'a [AppliableModification],
    positions: Vec<Reference>,
}

impl<'a> SAEVisitor for AutoRepealAccumulator<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if !self.ed_set.came_into_force_yesterday(position, self.date)? {
            return Ok(());
        }
        let mut add_it = if let Some(phrase) = &element.semantic_info.special_phrase {
            // Simple match isntead of matches! to make sure all cases are covered
            match phrase {
                SpecialPhrase::ArticleTitleAmendment(_)
                | SpecialPhrase::BlockAmendment(_)
                | SpecialPhrase::Repeal(_)
                | SpecialPhrase::TextAmendment(_)
                | SpecialPhrase::StructuralBlockAmendment(_)
                | SpecialPhrase::StructuralRepeal(_) => true,
                // Does not need to be auto-repealed
                SpecialPhrase::EnforcementDate(_) => false,
            }
        } else {
            false
        };

        for fixup in self.fixups {
            if fixup.source.as_ref().map_or(false, |s| s == position) {
                add_it = true;
            }
        }

        if add_it {
            self.positions.push(position.clone())
        }
        Ok(())
    }
}

impl<'a> AutoRepealAccumulator<'a> {
    pub fn new(
        ed_set: &'a EnforcementDateSet,
        date: NaiveDate,
        fixups: &'a [AppliableModification],
    ) -> Self {
        Self {
            ed_set,
            date,
            fixups,
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
