// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct NaiveDateRange {
    from: NaiveDate,
    to: NaiveDate,
}

/// Half-open range of NaiveDate-s, meant to be an exact replacement for Range<NaiveDate>
impl NaiveDateRange {
    pub fn new(from: NaiveDate, to: NaiveDate) -> Self {
        Self { from, to }
    }
}

impl Iterator for NaiveDateRange {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from >= self.to {
            return None;
        }
        let result = self.from;
        self.from = self.from.succ();
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_date_range() {
        let range = NaiveDateRange::new(
            NaiveDate::from_ymd(2020, 2, 27),
            NaiveDate::from_ymd(2020, 3, 3),
        );
        let dates_in_range: Vec<_> = range.collect();
        assert_eq!(
            dates_in_range,
            vec![
                NaiveDate::from_ymd(2020, 2, 27),
                NaiveDate::from_ymd(2020, 2, 28),
                NaiveDate::from_ymd(2020, 2, 29),
                NaiveDate::from_ymd(2020, 3, 1),
                NaiveDate::from_ymd(2020, 3, 2),
            ]
        )
    }
}
