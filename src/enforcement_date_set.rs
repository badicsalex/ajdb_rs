// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::ops::Add;

use anyhow::{bail, Result};
use chrono::{Datelike, NaiveDate};
use hun_law::{
    reference::Reference,
    semantic_info::{EnforcementDate, SemanticInfo, SpecialPhrase},
    structure::Act,
    util::walker::{SAEVisitor, WalkSAE},
};

pub struct ActualEnforcementDate {
    positions: Vec<Reference>,
    date: NaiveDate,
}

pub struct EnforcementDateSet {
    default_date: NaiveDate,
    enforcement_dates: Vec<ActualEnforcementDate>,
}

impl EnforcementDateSet {
    pub fn from_act(act: &Act) -> Result<Self> {
        let mut visitor = EnforcementDateAccumulator::default();
        act.walk_saes(&mut visitor)?;
        let raw_enforcement_dates = visitor.result;
        let default_dates: Vec<_> = raw_enforcement_dates
            .iter()
            .filter(|d| d.positions.is_empty())
            .collect();
        if default_dates.is_empty() {
            bail!(
                "Could not find the default enforcement date (out of {})",
                raw_enforcement_dates.len()
            );
        }
        if default_dates.len() > 1 {
            bail!(
                "Found too many default enforcement dates ({} out of {})",
                default_dates.len(),
                raw_enforcement_dates.len()
            );
        }
        let default_date = ActualEnforcementDate::from_enforcement_date(default_dates[0], act.publication_date)?.date;
        let enforcement_dates = raw_enforcement_dates
            .into_iter()
            .filter(|d| !d.positions.is_empty())
            .map(|d| ActualEnforcementDate::from_enforcement_date(&d, act.publication_date))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            default_date,
            enforcement_dates,
        })
    }

    pub fn effective_enforcement_date(&self, position: &Reference) -> NaiveDate {
        let mut result = self.default_date;
        for ed in &self.enforcement_dates {
            for ed_pos in &ed.positions{
                if ed_pos.contains(position) {
                    result = ed.date;
                }
            }
        }
        result
    }

    pub fn is_in_force(&self, position: &Reference, on_date: NaiveDate) -> bool {
        self.effective_enforcement_date(position) <= on_date
    }

    pub fn came_into_force_today(&self, position: &Reference, on_date: NaiveDate) -> bool {
        self.effective_enforcement_date(position) == on_date
    }
}

#[derive(Debug, Default)]
struct EnforcementDateAccumulator {
    result: Vec<EnforcementDate>,
}

impl SAEVisitor for EnforcementDateAccumulator {
    // on_enter and on_exit not needed, since EnforcementDates are always in leaf nodes.
    fn on_text(&mut self, _text: &String, semantic_info: &SemanticInfo) -> Result<()> {
        if let Some(SpecialPhrase::EnforcementDate(ed)) = &semantic_info.special_phrase {
            self.result.push(ed.clone())
        }
        Ok(())
    }
}

impl ActualEnforcementDate {
    pub fn from_enforcement_date(
        ed: &EnforcementDate,
        publication_date: NaiveDate,
    ) -> Result<Self> {
        let date = match ed.date {
            hun_law::semantic_info::EnforcementDateType::Date(d) => d,
            hun_law::semantic_info::EnforcementDateType::DaysAfterPublication(num_days) => {
                publication_date + chrono::Duration::days(num_days as i64)
            }
            hun_law::semantic_info::EnforcementDateType::DayInMonthAfterPublication {
                month,
                day,
            } => {
                let month_after_publication =
                    publication_date + chrono::Months::new(month.unwrap_or(1) as u32);
                NaiveDate::from_ymd(
                    month_after_publication.year(),
                    month_after_publication.month(),
                    day as u32,
                )
            }
            hun_law::semantic_info::EnforcementDateType::Special(_) => todo!(),
        };
        Ok(Self {
            positions: ed.positions.clone(),
            date,
        })
    }
}
