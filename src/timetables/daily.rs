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

use super::{
    generic_timetables::{Position, Timetable, Timetables, Vehicle, VehicleTimesError},
    iters::{PositionsIter, TimetableIter, VehicleIter},
    FlowDirection, Stop, TimetablesIter,
};
use crate::loads_data::Load;
use crate::{
    loads_data::LoadsData,
    time::{
        Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    },
    timetables::{
        generic_timetables::VehicleDataTrait, Timetables as TimetablesTrait,
        Types as TimetablesTypes,
    },
    transit_data::{Idx, VehicleJourney},
};
use chrono::NaiveDate;
use tracing::{trace, warn};

use core::cmp;
pub type Time = SecondsSinceDatasetUTCStart;

#[derive(Debug)]
pub struct DailyTimetables {
    timetables: Timetables<Time, Load, (), VehicleData>,
    calendar: Calendar,
}

#[derive(Clone, Debug)]
pub struct VehicleData {
    vehicle_journey_idx: Idx<VehicleJourney>,
    day: DaysSinceDatasetStart,
}

impl VehicleDataTrait for VehicleData {
    fn get_vehicle_journey_idx(&self) -> Idx<VehicleJourney> {
        self.vehicle_journey_idx
    }
}

impl TimetablesTypes for DailyTimetables {
    type Mission = Timetable;
    type Position = Position;
    type Trip = Vehicle;
    type VehicleData = VehicleData;
}

impl TimetablesTrait for DailyTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        Self {
            timetables: Timetables::new(),
            calendar: Calendar::new(first_date, last_date),
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

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        let day = self.timetables.vehicle_data(trip).day;
        self.calendar().to_naive_date(&day)
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

    fn previous_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.previous_position(position, mission)
    }

    fn arrival_time_of(&self, trip: &Self::Trip, position: &Self::Position) -> (Time, Load) {
        let (time, load) = self.timetables.arrival_time(trip, position);
        (*time, *load)
    }

    fn departure_time_of(&self, trip: &Self::Trip, position: &Self::Position) -> (Time, Load) {
        let (time, load) = self.timetables.departure_time(trip, position);
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
            .earliest_vehicle_to_board(waiting_time, mission, position)
            .map(|(trip, time, load)| (trip, *time, *load))
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, Time, Load)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        self.timetables
            .earliest_filtered_vehicle_to_board(waiting_time, mission, position, filter)
            .map(|(trip, time, load)| (trip, *time, *load))
    }

    fn latest_trip_that_debark_at(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .latest_vehicle_that_debark(time, mission, position)
            .map(|(trip, time, load)| (trip, *time, *load))
    }

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&Self::VehicleData) -> bool,
    {
        self.timetables
            .latest_filtered_vehicle_that_debark(time, mission, position, filter)
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
        let default_loads = vec![Load::default(); cmp::max(nb_of_positions - 1, 0)];

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
                    let insert_error = self.timetables.insert(
                        stops.clone(),
                        flows.clone(),
                        board_times_utc,
                        debark_times_utc,
                        loads.iter().copied(),
                        (),
                        vehicle_data,
                    );
                    match insert_error {
                        Ok(mission) => {
                            result.push(mission);
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
