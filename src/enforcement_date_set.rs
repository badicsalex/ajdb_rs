// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;
use hun_law::{reference::Reference, semantic_info::EnforcementDate, structure::Act};

pub struct ActualEnforcementDate {
    positions: Vec<Reference>,
    date: NaiveDate,
}

pub struct EnforcementDateSet {
    default_date: NaiveDate,
    enforcement_dates: Vec<ActualEnforcementDate>,
}

impl EnforcementDateSet {
    pub fn from_act(act: &Act) -> Self {
        todo!()
    }

    pub fn is_in_force(&self, position: Reference) -> bool {
        todo!()
    }

    pub fn came_into_force_today(&self, position: Reference) -> bool {
        todo!()
    }
}

impl ActualEnforcementDate {
    pub fn from_enforcement_date(date: &EnforcementDate) -> Self {
        match date.date {
            hun_law::semantic_info::EnforcementDateType::Date(_) => todo!(),
            hun_law::semantic_info::EnforcementDateType::DaysAfterPublication(_) => todo!(),
            hun_law::semantic_info::EnforcementDateType::DayInMonthAfterPublication {
                month,
                day,
            } => todo!(),
            hun_law::semantic_info::EnforcementDateType::Special(_) => todo!(),
        }
    }
}
