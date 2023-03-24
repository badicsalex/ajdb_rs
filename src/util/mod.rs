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

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

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

pub fn read_all(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let mut result = Vec::new();
    File::open(path)?.read_to_end(&mut result)?;
    Ok(result)
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
