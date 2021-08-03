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


use crate::{time::{Calendar, DaysSinceDatasetStart, days_patterns::{DaysPattern, DaysPatterns}}};

use super::{ generic_timetables::{Timetable}};





#[derive(Debug)]
pub struct DayToTimetable {
    // invariants : 
    //  1. a day is set in at most one DaysPattern of the Vec
    //  2. a timetable appears at most once in the vec
    pattern_timetables :  Vec<(DaysPattern, Timetable)>,
}


impl DayToTimetable {
    pub fn new(calendar : & Calendar) -> Self {
        Self {
            pattern_timetables : Vec::new()
        }
    }

    pub fn insert(& mut self, 
        day_to_insert : & DaysSinceDatasetStart, 
        timetable_to_insert : & Timetable, 
        days_patterns : & mut DaysPatterns, 
        calendar : & Calendar
    ) -> Result<(), InsertError>
    {
        // let's check if this day is already set
        for (days_pattern, _) in self.pattern_timetables.iter() {
            if days_patterns.is_allowed(days_pattern, day_to_insert) {
                return Err(InsertError::DayAlreadySet);
            }
        }

        // We try to find the first element whose timetable contains timetable_to_insert .
        // Because of our invariant 2., if such an element is found we know that 
        // timetable_to_insert does not appears in any other element of the vec.
        let has_days_pattern = self.pattern_timetables.iter_mut().
            find(|(days_pattern, timetable)| {
                    timetable == timetable_to_insert
                })
            .map(|(days_pattern, _)| days_pattern); // we are just interested in the pattern

        if let Some(old_days_pattern) = has_days_pattern {
            // so now timetable_to_insert is valid on old_days_pattern and day_to_insert
            // let's create a new days_pattern for that
            let new_days_pattern = days_patterns
            .get_pattern_with_additional_day(*old_days_pattern, day_to_insert, calendar)
            .map_err(|()| InsertError::DayAlreadySet)?;

            * old_days_pattern = new_days_pattern;
        }
        else {  // if timetable_to_insert does not appears in the Vec, 
                // let's push a new element to the Vec with it

            let days_pattern = days_patterns.get_for_day(day_to_insert, calendar);
            self.pattern_timetables.push((days_pattern, *timetable_to_insert));
        }
        
        Ok(())
    }

    pub fn remove(& mut self,
        day_to_remove : & DaysSinceDatasetStart,
        days_patterns : & mut DaysPatterns, 
        calendar : & Calendar
    ) ->Result<(), RemoveError>
    {
        // let's try to find the first element where day_to_remove is set.
        // Because of invariant 1., if such an element is found, we know that
        // day_to_remove is not set in any other element
        let has_days_pattern = self.pattern_timetables.iter_mut()
            .map(|(days_pattern, _)| days_pattern)
            .enumerate()
            .find(|(idx, days_pattern)| {
                    days_patterns.is_allowed(days_pattern, day_to_remove)
                });


        if let Some((idx, old_days_pattern)) = has_days_pattern{

            let new_days_pattern = days_patterns.get_pattern_without_day(*old_days_pattern, day_to_remove, calendar)
                .map_err(|()| RemoveError::DayNotSet)?;

            if days_patterns.is_empty_pattern(&new_days_pattern) {
                self.pattern_timetables.swap_remove(idx);
                Ok(())
            }
            else {
                *old_days_pattern = new_days_pattern;
                Ok(())
            }

        }
        else {
            Err(RemoveError::DayNotSet)
        }
    }

    fn get_timetable_for(&self, day : & DaysSinceDatasetStart, days_patterns : & DaysPatterns) -> Option<Timetable> {
        self.pattern_timetables.iter()
            .find(|(days_pattern, timetable)| {
                    days_patterns.is_allowed(days_pattern, day)
                })
            .map(|(days_pattern, timetable)| timetable.clone())
    }
}

// fn get_timetable(day) -> Option<Timetable>
// fn add_timetable(day, timetable)  // update the struct, insert
// fn remove(day)

enum InsertError {
    DayAlreadySet,
}

enum RemoveError {
    DayNotSet,
}
