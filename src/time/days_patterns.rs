// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use std::iter::Enumerate;

use crate::time::{Calendar, DaysSinceDatasetStart};
use chrono::NaiveDate;

#[derive(Debug)]
pub struct DaysPatterns {
    days_patterns: Vec<DaysPatternData>,

    buffer: Vec<bool>,
}
#[derive(Debug)]
struct DaysPatternData {
    allowed_dates: Vec<bool>,
}

#[derive(Debug, Copy, Clone)]
pub struct DaysPattern {
    idx: usize,
}

impl DaysPatterns {
    pub fn new(nb_of_days: usize) -> Self {
        Self {
            days_patterns: Vec::new(),
            buffer: vec![false; nb_of_days],
        }
    }

    pub fn is_allowed(&self, days_pattern: &DaysPattern, day: &DaysSinceDatasetStart) -> bool {
        debug_assert!((day.days as usize) < self.buffer.len());
        debug_assert!(days_pattern.idx < self.days_patterns.len());
        let day_idx: usize = day.days.into();
        self.days_patterns[days_pattern.idx].allowed_dates[day_idx]
    }

    pub fn days_in_pattern(&self, days_pattern: &DaysPattern) -> DaysInPatternIter {
        let iter = self.days_patterns[days_pattern.idx]
            .allowed_dates
            .iter()
            .enumerate();
        DaysInPatternIter {
            allowed_dates: iter,
        }
    }

    pub fn get_or_insert<'a, Dates>(&mut self, dates: Dates, calendar: &Calendar) -> DaysPattern
    where
        Dates: Iterator<Item = &'a NaiveDate>,
    {
        // set all elements of the buffer to false
        for val in self.buffer.iter_mut() {
            *val = false
        }

        for date in dates {
            let has_offset = calendar.date_to_offset(date);
            if let Some(offset) = has_offset {
                self.buffer[offset as usize] = true;
            }
        }

        let has_days_pattern = self
            .days_patterns
            .iter()
            .enumerate()
            .find(|(_, calendar)| calendar.allowed_dates == self.buffer)
            .map(|(idx, _)| idx);

        let idx = if let Some(idx) = has_days_pattern {
            idx
        } else {
            let idx = self.days_patterns.len();
            let days_pattern_data = DaysPatternData {
                allowed_dates: self.buffer.clone(),
            };
            self.days_patterns.push(days_pattern_data);
            idx
        };

        DaysPattern { idx }
    }

    pub fn common_days(&self, first_pattern : & DaysPattern, second_pattern : & DaysPattern, calendar : & Calendar) -> Vec<DaysSinceDatasetStart> {
        let first_data = &self.days_patterns[first_pattern.idx].allowed_dates;
        let second_data = &self.days_patterns[second_pattern.idx].allowed_dates;
        let days = calendar.days();
        let mut result = Vec::new();
        for (day, (first, second)) in days.zip(first_data.iter().zip(second_data.iter())) {
            if *first && *second { 
                result.push(day);
            }
        }
        result
    }
}

pub struct DaysInPatternIter<'pattern> {
    allowed_dates: Enumerate<std::slice::Iter<'pattern, bool>>,
}

impl<'pattern> Iterator for DaysInPatternIter<'pattern> {
    type Item = DaysSinceDatasetStart;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.allowed_dates.next() {
                Some((day_idx, is_allowed)) if *is_allowed => {
                    let days: u16 = day_idx as u16;
                    return Some(DaysSinceDatasetStart { days });
                }
                Some(_) => (),
                None => {
                    return None;
                }
            }
        }
    }
}
