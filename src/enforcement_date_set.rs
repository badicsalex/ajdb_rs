// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{ensure, Result};
use chrono::{Datelike, NaiveDate};
use hun_law::{
    identifier::IdentifierCommon,
    reference::{parts::AnyReferencePart, Reference},
    semantic_info::{EnforcementDate, SpecialPhrase},
    structure::{Act, ChildrenCommon, SubArticleElement},
    util::{debug::WithElemContext, walker::SAEVisitor},
};
use log::info;

use crate::fixups::ActFixups;

#[derive(Debug)]
pub struct ActualEnforcementDate {
    positions: Vec<Reference>,
    date: NaiveDate,
}

#[derive(Debug)]
pub struct EnforcementDateSet {
    default_date: NaiveDate,
    // TODO: this needs a faster data structure to prevent two levels of linear searches
    enforcement_dates: Vec<ActualEnforcementDate>,
}

impl EnforcementDateSet {
    pub fn from_act(act: &Act) -> Result<Self> {
        let mut visitor = EnforcementDateAccumulator::default();
        act.walk_saes(&mut visitor)
            .with_elem_context("Getting enforcement dates failed", act)?;
        let additional_eds = ActFixups::load(act.identifier)?.get_additional_enforcement_dates();
        if !additional_eds.is_empty() {
            info!(
                "Fixup: Using {} additional enforcement dates",
                additional_eds.len()
            );
            visitor.result.extend(additional_eds);
        }
        Self::from_enforcement_dates(&visitor.result, act.publication_date)
            .with_elem_context("Calculating enforcement dates failed", act)
    }
    pub fn from_enforcement_dates(
        raw_enforcement_dates: &[EnforcementDate],
        publication_date: NaiveDate,
    ) -> Result<Self> {
        let default_dates: Vec<_> = raw_enforcement_dates
            .iter()
            .filter(|d| d.positions.is_empty())
            .collect();
        ensure!(
            !default_dates.is_empty(),
            "Could not find the default enforcement date (out of {})",
            raw_enforcement_dates.len()
        );
        ensure!(
            default_dates.len() == 1,
            "Found too many default enforcement dates ({} out of {})",
            default_dates.len(),
            raw_enforcement_dates.len()
        );
        let default_date =
            ActualEnforcementDate::from_enforcement_date(default_dates[0], publication_date)?.date;
        let enforcement_dates = raw_enforcement_dates
            .iter()
            .filter(|d| !d.positions.is_empty())
            .map(|d| ActualEnforcementDate::from_enforcement_date(d, publication_date))
            .collect::<Result<Vec<_>>>()?;

        // See 61/2009. (XII. 14.) IRM rendelet 81. § (2)
        ensure!(
            enforcement_dates.iter().all(|ed| ed.date >= default_date),
            "Some enforcement dates found after the act's default date ({default_date}): {:?}",
            enforcement_dates
                .iter()
                .filter(|ed| ed.date < default_date)
                .collect::<Vec<_>>(),
        );

        Ok(Self {
            default_date,
            enforcement_dates,
        })
    }

    /// Check the enforcement date of the reference.
    pub fn effective_enforcement_date(&self, position: &Reference) -> NaiveDate {
        // TODO: Check the act instead
        let position = position.without_act();
        let mut result = self.default_date;
        for ed in &self.enforcement_dates {
            for ed_pos in &ed.positions {
                if ed_pos.contains(&position) {
                    result = ed.date;
                }
            }
        }
        result
    }

    /// Returns None for elements that are not specifically mentioned (e.g. the children of mentioned elements)
    /// Returns the enforcement date of elements that are mentioned and not in force
    pub fn specific_element_not_in_force(
        &self,
        position: &Reference,
        on_date: NaiveDate,
    ) -> Option<NaiveDate> {
        // TODO: Check the act instead
        let position = position.without_act();
        let last_part = position.get_last_part();
        // TODO: speed this up with a hashmap if it's a performance problem
        self.enforcement_dates
            .iter()
            .find(|ed| {
                ed.date > on_date
                    && ed.positions.iter().any(|p| {
                        // This is needed instead of a simple == to handle ranges.
                        is_same_level(&last_part, &p.get_last_part()) && p.contains(&position)
                    })
            })
            .map(|ed| ed.date)
    }

    pub fn is_in_force(&self, position: &Reference, on_date: NaiveDate) -> bool {
        // TODO: short circuit trivial case when all dates are in the past
        self.effective_enforcement_date(position) <= on_date
    }

    pub fn came_into_force_today(&self, position: &Reference, on_date: NaiveDate) -> bool {
        // TODO: short circuit trivial cases when no dates are "on_date"
        self.effective_enforcement_date(position) == on_date
    }

    pub fn came_into_force_yesterday(&self, position: &Reference, on_date: NaiveDate) -> bool {
        // TODO: short circuit trivial cases when no dates are "on_date"
        self.effective_enforcement_date(position) == on_date.pred()
    }

    pub fn get_all_dates(&self) -> Vec<NaiveDate> {
        let mut result: Vec<_> = self.enforcement_dates.iter().map(|ed| ed.date).collect();
        result.push(self.default_date);
        result
    }
}

#[derive(Debug, Default)]
struct EnforcementDateAccumulator {
    result: Vec<EnforcementDate>,
}

impl SAEVisitor for EnforcementDateAccumulator {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        _position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if let Some(SpecialPhrase::EnforcementDate(ed)) = &element.semantic_info.special_phrase {
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
        ensure!(
            ed.positions.iter().all(|p| p.act().is_none()),
            "Reference contained act in from_enforcement_date"
        );
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
        };
        Ok(Self {
            positions: ed.positions.clone(),
            date,
        })
    }
}

#[allow(clippy::match_like_matches_macro)]
fn is_same_level(a: &AnyReferencePart, b: &AnyReferencePart) -> bool {
    match (a, b) {
        (AnyReferencePart::Empty, AnyReferencePart::Empty) => true,
        (AnyReferencePart::Act(_), AnyReferencePart::Act(_)) => true,
        (AnyReferencePart::Article(_), AnyReferencePart::Article(_)) => true,
        (AnyReferencePart::Paragraph(_), AnyReferencePart::Paragraph(_)) => true,
        (AnyReferencePart::Point(_), AnyReferencePart::Point(_)) => true,
        (AnyReferencePart::Subpoint(_), AnyReferencePart::Subpoint(_)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use hun_law::util::singleton_yaml;
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
    struct TestRef {
        position: Reference,
        date: NaiveDate,
    }

    const TEST_ED_SET: &str = r#"
        - date:
            Date: 2013-07-15
        - date:
            Date: 2013-11-02
          positions:
            - article: "180"
              paragraph: "1"
              point: "a"
        - date:
            Date: 2014-09-01
          positions:
            - article:
                start: "55"
                end: "66"
            - article: "70"
            - article: "72"
              point: a
            - article: "73"
              point: a
            - article: "73"
              point: b
        - date:
            DayInMonthAfterPublication:
              day: 1
          positions:
            - article: "38"
        - date:
            DayInMonthAfterPublication:
              month: 2
              day: 5
          positions:
            - article: "39"
        - date:
            DaysAfterPublication: 30
          positions:
            - article: "40"
    "#;

    const TEST_REFS: &str = r#"
        - position:
            article: '1'
          date: 2013-07-15
        - position:
            article: '180'
            paragraph: '1'
          date: 2013-07-15
        - position:
            article: '180'
            paragraph: '1'
          date: 2013-07-15
        - position:
            article: '180'
            paragraph: '1'
            point: 'a'
          date: 2013-11-02
        - position:
            article: '180'
            paragraph: '1'
            point: 'a'
            subpoint: 'ab'
          date: 2013-11-02
        - position:
            article: "60"
            paragraph: "5"
          date: 2014-09-01
        - position:
            article: "73"
            point: "b"
          date: 2014-09-01
        - position:
            article: "38"
          date: 2013-08-01
        - position:
            article: "39"
          date: 2013-09-05
        - position:
            article: "40"
          date: 2013-07-31
    "#;

    #[test]
    fn test_enforcement_date_set() {
        let enforcement_dates: Vec<EnforcementDate> =
            singleton_yaml::from_str(TEST_ED_SET).unwrap();
        let test_refs: Vec<TestRef> = singleton_yaml::from_str(TEST_REFS).unwrap();
        let ed_set = EnforcementDateSet::from_enforcement_dates(
            &enforcement_dates,
            NaiveDate::from_ymd(2013, 7, 1),
        )
        .unwrap();

        for test_ref in test_refs {
            let effective = TestRef {
                position: test_ref.position.clone(),
                date: ed_set.effective_enforcement_date(&test_ref.position),
            };
            assert_eq!(
                singleton_yaml::to_string(&test_ref).unwrap(),
                singleton_yaml::to_string(&effective).unwrap()
            );
        }
    }
}
