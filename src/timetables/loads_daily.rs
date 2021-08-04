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

use std::collections::BTreeMap;

use super::{FlowDirection, Stop, TimetablesIter, day_to_timetable::DayToTimetable, generic_timetables::{Position, Timetable, Timetables, Vehicle, VehicleTimesError}, iters::{PositionsIter, TimetableIter, VehicleIter}};

use crate::{time::days_patterns::DaysPatterns, transit_data::{Idx, VehicleJourney}};
use crate::{
    loads_data::LoadsData,
    time::{
        Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    },
};
use chrono::NaiveDate;

use crate::timetables::{Timetables as TimetablesTrait, Types as TimetablesTypes};

use crate::log::{trace, warn};

use crate::loads_data::Load;

pub type Time = SecondsSinceDatasetUTCStart;
#[derive(Debug)]
pub struct DailyTimetables {
    timetables: Timetables<Time, Load, (), VehicleData>,
    calendar: Calendar,
    days_patterns: DaysPatterns,
    vehicle_journey_to_timetables : BTreeMap<Idx<VehicleJourney>, DayToTimetable>,

}
#[derive(Clone, Debug)]
struct VehicleData {
    vehicle_journey_idx: Idx<VehicleJourney>,
    day: DaysSinceDatasetStart,
}

impl TimetablesTypes for DailyTimetables {
    type Mission = Timetable;

    type Position = Position;

    type Trip = Vehicle;
}

impl TimetablesTrait for DailyTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        let calendar = Calendar::new(first_date, last_date);
        let nb_of_days: usize = calendar.nb_of_days().into();
        Self {
            timetables: Timetables::new(),
            calendar: calendar,
            days_patterns: DaysPatterns::new(nb_of_days),
            vehicle_journey_to_timetables : BTreeMap::new(),
        }
    }

    fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        mission.idx
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney> {
        self.timetables.vehicle_data(trip).vehicle_journey_idx
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        let day = self.timetables.vehicle_data(trip).day;
        self.calendar().to_naive_date(&day)
    }

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(trip)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop {
        *self.timetables.stop_at(position, mission)
    }

    fn nb_of_trips(&self) -> usize {
        self.timetables.nb_of_trips()
    }

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables.is_upstream(upstream, downstream, mission)
    }

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn arrival_time_of(&self, trip: &Self::Trip, position: &Self::Position) -> (Time, Load) {
        let (time, load) = self.timetables.arrival_time(trip, position);
        (*time, *load)
    }

    fn debark_time_of(&self, trip: &Self::Trip, position: &Self::Position) -> Option<(Time, Load)> {
        self.timetables
            .debark_time(trip, position)
            .map(|(time, load)| (*time, *load))
    }

    fn board_time_of(&self, trip: &Self::Trip, position: &Self::Position) -> Option<(Time, Load)> {
        self.timetables
            .board_time(trip, position)
            .map(|(time, load)| (*time, *load))
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, Time, Load)> {
        self.timetables
            .earliest_vehicle_to_board(&waiting_time, mission, position)
            .map(|(trip, time, load)| (trip, *time, *load))
    }

    fn insert<'date, Stops, Flows, Dates, Times>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: Times,
        debark_times: Times,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate>,
        Times: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut result = Vec::new();
        let nb_of_positions = stops.len();
        let default_loads = if nb_of_positions > 0 {
            vec![Load::default(); nb_of_positions - 1]
        } else {
            vec![Load::default(); 0]
        };

        let vj_timetables = self.vehicle_journey_to_timetables
            .entry(vehicle_journey_idx)
            .or_insert(DayToTimetable::new(&self.calendar));

        for date in valid_dates {
            let has_day = self.calendar.date_to_days_since_start(date);
            match has_day {
                None => {
                    trace!(
                        "Skipping vehicle journey {} on day {} because  \
                        this day is not allowed by the calendar. \
                        Allowed day are between {} and {}",
                        vehicle_journey.id,
                        date,
                        self.calendar.first_date(),
                        self.calendar.last_date(),
                    );
                    continue;
                }
                Some(day) => {

                    if vj_timetables.contains_day(&day, &self.days_patterns)  {
                        warn!("Trying to add vehicle journey {} multiple time for day {}. Insertion skipped.",
                            vehicle_journey.id,
                            self.calendar.to_naive_date(&day)
                        );
                        continue;
                    }

                    let calendar = &self.calendar;
                    let board_times_utc = board_times
                        .clone()
                        .map(|time| calendar.compose(&day, &time, timezone));
                    let debark_times_utc = debark_times
                        .clone()
                        .map(|time| calendar.compose(&day, &time, timezone));

                    let loads = loads_data
                        .loads(&vehicle_journey_idx, date)
                        .unwrap_or_else(|| default_loads.as_slice());
                    let vehicle_data = VehicleData {
                        vehicle_journey_idx,
                        day,
                    };
                    let insert_result = self.timetables.insert(
                        stops.clone(),
                        flows.clone(),
                        board_times_utc,
                        debark_times_utc,
                        loads.iter().copied(),
                        (),
                        vehicle_data,
                    );
                    match insert_result {
                        Ok(mission) => {
                            if ! result.contains(&mission) {
                                result.push(mission.clone());
                            }
                            let days_pattern = self.days_patterns.get_for_day(&day, &self.calendar);
                            vj_timetables.insert_days_pattern(&days_pattern, &mission, & mut self.days_patterns, &self.calendar)
                                .unwrap(); // unwrap should be safe here, because we check above that vj_timetables has no intersection with days_pattern

                        }
                        Err(error) => {
                            handle_vehicletimes_error(vehicle_journey, date, &error);
                        }
                    }
                }
            }
        }
        result
    }

    fn remove(
        &mut self,
        date: & chrono::NaiveDate,
        vehicle_journey_idx: Idx<VehicleJourney>,
    ) -> Result<(), super::RemovalError> {
        todo!()
    }
}

impl<'a> TimetablesIter<'a> for DailyTimetables {
    type Positions = PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = VehicleIter;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        self.timetables.vehicles(mission)
    }

    type Missions = TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

fn handle_vehicletimes_error(
    vehicle_journey: &VehicleJourney,
    date: &NaiveDate,
    error: &VehicleTimesError,
) {
    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {} on day {} because its \
                    debark time at : \n {:?} \n\
                    is earlier than its \
                    board time upstream at : \n {:?} \n. ",
                vehicle_journey.id, date, downstream_stop_time, upstream_stop_time
            );
        }
        VehicleTimesError::DecreasingBoardTime(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {} on day {} because its \
                    board time at : \n {:?} \n \
                    is earlier than its \
                    board time upstream at : \n {:?} \n. ",
                vehicle_journey.id, date, downstream_stop_time, upstream_stop_time
            );
        }
        VehicleTimesError::DecreasingDebarkTime(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {} on day {} because its \
                    debark time at : \n {:?} \n \
                    is earlier than its \
                    debark time upstream at : \n {:?} \n. ",
                vehicle_journey.id, date, downstream_stop_time, upstream_stop_time
            );
        }
    }
}
