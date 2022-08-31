// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;

pub struct NaiveDateRange {}

impl NaiveDateRange {
    pub fn new(from: NaiveDate, to: NaiveDate) -> Self {
        todo!()
    }
}

impl Iterator for NaiveDateRange {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
