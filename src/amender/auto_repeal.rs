// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{
    reference::Reference,
    semantic_info::{EnforcementDate, SemanticInfo, SpecialPhrase},
    util::walker::SAEVisitor,
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
    fn on_text(
        &mut self,
        position: &Reference,
        _text: &String,
        semantic_info: &SemanticInfo,
    ) -> Result<()> {
        self.repeal_one(position, semantic_info)?;
        self.parse_inline_repeal(semantic_info)
    }

    fn on_enter(
        &mut self,
        position: &Reference,
        _intro: &String,
        _wrap_up: &Option<String>,
        semantic_info: &SemanticInfo,
    ) -> Result<()> {
        self.repeal_one(position, semantic_info)
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
                Ok(SimplifiedRepeal {
                    position: p.relative_to(act_ref)?,
                    text: None,
                }
                .into())
            })
            .collect::<Result<Vec<_>>>()
    }

    fn repeal_one(&mut self, position: &Reference, semantic_info: &SemanticInfo) -> Result<()> {
        if self.ed_set.came_into_force_yesterday(position, self.date)? {
            if let Some(phrase) = &semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(_)
                    | SpecialPhrase::BlockAmendment(_)
                    | SpecialPhrase::Repeal(_)
                    | SpecialPhrase::TextAmendment(_)
                    | SpecialPhrase::StructuralBlockAmendment(_)
                    | SpecialPhrase::StructuralRepeal(_) => self.positions.push(position.clone()),
                    // Not a modification
                    SpecialPhrase::EnforcementDate(_) => (),
                }
            }
        }
        Ok(())
    }

    fn parse_inline_repeal(&mut self, semantic_info: &SemanticInfo) -> Result<()> {
        if let Some(SpecialPhrase::EnforcementDate(EnforcementDate {
            inline_repeal: Some(inline_repeal_date),
            ..
        })) = semantic_info.special_phrase
        {
            if inline_repeal_date == self.date {
                self.positions.push(Reference::default());
            }
        }
        Ok(())
    }
}
