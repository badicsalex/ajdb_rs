// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{bail, ensure, Context, Result};
use chrono::{Datelike, NaiveDate};
use hun_law::{
    identifier::IdentifierCommon,
    reference::Reference,
    semantic_info::{EnforcementDate, SpecialPhrase},
    structure::{Act, ChildrenCommon, SubArticleElement},
    util::{debug::WithElemContext, walker::SAEVisitor},
};

#[derive(Debug)]
pub struct ActualEnforcementDate {
    positions: Vec<Reference>,
    date: NaiveDate,
}

#[derive(Debug)]
pub struct EnforcementDateSet {
    default_date: NaiveDate,
    enforcement_dates: Vec<ActualEnforcementDate>,
}

impl EnforcementDateSet {
    pub fn from_act(act: &Act) -> Result<Self> {
        let mut visitor = EnforcementDateAccumulator::default();
        act.walk_saes(&mut visitor)
            .with_elem_context("Getting enforcement dates failed", act)?;
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
        let default_date =
            ActualEnforcementDate::from_enforcement_date(default_dates[0], publication_date)?.date;
        let enforcement_dates = raw_enforcement_dates
            .iter()
            .filter(|d| !d.positions.is_empty())
            .map(|d| ActualEnforcementDate::from_enforcement_date(d, publication_date))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            default_date,
            enforcement_dates,
        })
    }

    /// Check the enforcement date of the reference.
    pub fn effective_enforcement_date(&self, position: &Reference) -> Result<NaiveDate> {
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
        Ok(result)
    }

    pub fn is_in_force(&self, position: &Reference, on_date: NaiveDate) -> Result<bool> {
        // TODO: short circuit trivial case when all dates are in the past
        Ok(self
            .effective_enforcement_date(position)
            .with_context(|| "In is_in_force()")?
            <= on_date)
    }

    pub fn came_into_force_today(&self, position: &Reference, on_date: NaiveDate) -> Result<bool> {
        // TODO: short circuit trivial cases when no dates are "on_date"
        Ok(self
            .effective_enforcement_date(position)
            .with_context(|| "In came_into_force_today()")?
            == on_date)
    }

    pub fn came_into_force_yesterday(
        &self,
        position: &Reference,
        on_date: NaiveDate,
    ) -> Result<bool> {
        // TODO: short circuit trivial cases when no dates are "on_date"
        Ok(self
            .effective_enforcement_date(position)
            .with_context(|| "In came_into_force_today()")?
            == on_date.pred())
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
            Date: 2013-07-02
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
          date: 2013-07-02
        - position:
            article: '180'
            paragraph: '1'
          date: 2013-07-02
        - position:
            article: '180'
            paragraph: '1'
          date: 2013-07-02
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
          date: 2012-09-01
        - position:
            article: "39"
          date: 2012-10-05
        - position:
            article: "40"
          date: 2012-09-27
    "#;

    #[test]
    fn test_enforcement_date_set() {
        let enforcement_dates: Vec<EnforcementDate> =
            singleton_yaml::from_str(TEST_ED_SET).unwrap();
        let test_refs: Vec<TestRef> = singleton_yaml::from_str(TEST_REFS).unwrap();
        let ed_set = EnforcementDateSet::from_enforcement_dates(
            &enforcement_dates,
            NaiveDate::from_ymd(2012, 8, 28),
        )
        .unwrap();

        for test_ref in test_refs {
            let effective = TestRef {
                position: test_ref.position.clone(),
                date: ed_set
                    .effective_enforcement_date(&test_ref.position)
                    .unwrap(),
            };
            assert_eq!(
                singleton_yaml::to_string(&test_ref).unwrap(),
                singleton_yaml::to_string(&effective).unwrap()
            );
        }
    }
}
