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

use crate::{
    loads_data::{Load, LoadsData},
    time::days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns},
};

use super::{
    generic_timetables::{Timetables, Vehicle},
    TimetablesIter,
};

use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    SecondsSinceUTCDayStart, TimezonesPatterns,
};
use crate::transit_data::{Idx, VehicleJourney};
use chrono::NaiveDate;

use crate::timetables::{
    FlowDirection, Stop, Timetables as TimetablesTrait, Types as TimetablesTypes,
};

use crate::log::warn;
#[derive(Debug)]
pub struct PeriodicSplitVjByTzTimetables {
    timetables: Timetables<SecondsSinceUTCDayStart, Load, (), VehicleData>,
    calendar: Calendar,
    days_patterns: DaysPatterns,
    tz_patterns: TimezonesPatterns,
}

#[derive(Debug, Clone)]
pub struct VehicleData {
    days_pattern: DaysPattern,
    vehicle_journey_idx: Idx<VehicleJourney>,
}

impl VehicleDataTrait for VehicleData {
    fn get_vehicle_journey_idx(&self) -> Idx<VehicleJourney> {
        self.vehicle_journey_idx
    }
}

#[derive(Debug, Clone)]
pub struct Trip {
    vehicle: Vehicle,
    day: DaysSinceDatasetStart,
}

impl TimetablesTypes for PeriodicSplitVjByTzTimetables {
    type Mission = Timetable;
    type Position = Position;
    type Trip = Trip;
    type VehicleData = VehicleData;
}

impl TimetablesTrait for PeriodicSplitVjByTzTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        let calendar = Calendar::new(first_date, last_date);
        let nb_of_days: usize = calendar.nb_of_days().into();
        Self {
            timetables: Timetables::new(),
            calendar,
            days_patterns: DaysPatterns::new(nb_of_days),
            tz_patterns: TimezonesPatterns::new(),
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
        self.timetables
            .vehicle_data(&trip.vehicle)
            .vehicle_journey_idx
    }

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        self.calendar().to_naive_date(&trip.day)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(&trip.vehicle)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> super::Stop {
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

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.arrival_time(&trip.vehicle, position);
        let time_utc = self.calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.departure_time(&trip.vehicle, position);
        let time_utc = self.calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.debark_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = self.calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.board_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = self.calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.earliest_filtered_trip_to_board_at(
            waiting_time,
            mission,
            position,
            |_: &VehicleData| true,
        )
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let has_earliest_and_latest_board_time =
            self.timetables.earliest_and_latest_board_time(position);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (_earliest_board_time_in_day, _latest_board_time_in_day) =
            has_earliest_and_latest_board_time?;

        let decompositions = self.calendar.decompositions_utc(
            waiting_time,
            SecondsSinceUTCDayStart::max(),
            SecondsSinceUTCDayStart::min(),
        );

        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;

        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.earliest_filtered_vehicle_to_board(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    filter(vehicle_data)
                        && self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, arrival_time_in_day_at_next_stop, load)) = has_vehicle {
                let arrival_time_at_next_stop = self
                    .calendar
                    .compose_utc(&waiting_day, arrival_time_in_day_at_next_stop);

                if let Some((_, _, best_arrival_time, best_load)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if arrival_time_at_next_stop < *best_arrival_time
                        || (arrival_time_at_next_stop == *best_arrival_time && load < best_load)
                    {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, arrival_time_at_next_stop, *load));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, arrival_time_at_next_stop, *load));
                }
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position.map(
            |(vehicle, day, arrival_time_at_next_stop, load)| {
                let trip = Trip { vehicle, day };
                (trip, arrival_time_at_next_stop, load)
            },
        )
    }

    fn latest_trip_that_debark_at(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let has_earliest_and_latest_debark_time =
            self.timetables.earliest_and_latest_debark_time(position);

        // if there is no earliest/latest debark time, it means that this position cannot be debarked
        // and we return None
        let (_earliest_debark_time_in_day, _latest_debark_time_in_day) =
            has_earliest_and_latest_debark_time?;

        let decompositions = self.calendar.decompositions_utc(
            time,
            SecondsSinceUTCDayStart::max(),
            SecondsSinceUTCDayStart::min(),
            // *latest_debark_time_in_day,
            // *earliest_debark_time_in_day,
        );
        let mut best_vehicle_day_and_its_departure_time_at_previous_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.latest_filtered_vehicle_that_debark(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, departure_time_in_day_at_previous_stop, load)) = has_vehicle {
                let departure_time_at_previous_stop = self
                    .calendar
                    .compose_utc(&waiting_day, departure_time_in_day_at_previous_stop);

                if let Some((_, _, best_departure_time, best_load)) =
                    &best_vehicle_day_and_its_departure_time_at_previous_position
                {
                    if departure_time_at_previous_stop >= *best_departure_time
                        || (departure_time_at_previous_stop == *best_departure_time
                            && load < best_load)
                    {
                        best_vehicle_day_and_its_departure_time_at_previous_position =
                            Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                    }
                } else {
                    best_vehicle_day_and_its_departure_time_at_previous_position =
                        Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                }
            }
        }

        best_vehicle_day_and_its_departure_time_at_previous_position.map(
            |(vehicle, day, departure_time_at_previous_stop, load)| {
                let trip = Trip { vehicle, day };
                (trip, departure_time_at_previous_stop, load)
            },
        )
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
        let has_earliest_and_latest_debark_time =
            self.timetables.earliest_and_latest_debark_time(position);

        // if there is no earliest/latest debark time, it means that this position cannot be debarked
        // and we return None
        let (_earliest_debark_time_in_day, _latest_debark_time_in_day) =
            has_earliest_and_latest_debark_time?;

        let decompositions = self.calendar.decompositions_utc(
            time,
            SecondsSinceUTCDayStart::max(),
            SecondsSinceUTCDayStart::min(),
            // *latest_debark_time_in_day,
            // *earliest_debark_time_in_day,
        );
        let mut best_vehicle_day_and_its_departure_time_at_previous_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.latest_filtered_vehicle_that_debark(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    filter(vehicle_data)
                        && self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, departure_time_in_day_at_previous_stop, load)) = has_vehicle {
                let departure_time_at_previous_stop = self
                    .calendar
                    .compose_utc(&waiting_day, departure_time_in_day_at_previous_stop);

                if let Some((_, _, best_departure_time, best_load)) =
                    &best_vehicle_day_and_its_departure_time_at_previous_position
                {
                    if departure_time_at_previous_stop >= *best_departure_time
                        || (departure_time_at_previous_stop == *best_departure_time
                            && load < best_load)
                    {
                        best_vehicle_day_and_its_departure_time_at_previous_position =
                            Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                    }
                } else {
                    best_vehicle_day_and_its_departure_time_at_previous_position =
                        Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                }
            }
        }

        best_vehicle_day_and_its_departure_time_at_previous_position.map(
            |(vehicle, day, departure_time_at_previous_stop, load)| {
                let trip = Trip { vehicle, day };
                (trip, departure_time_at_previous_stop, load)
            },
        )
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
        let mut load_patterns_dates: BTreeMap<&[Load], Vec<NaiveDate>> = BTreeMap::new();

        let nb_of_positions = stops.len();
        let default_loads = vec![Load::default(); cmp::max(nb_of_positions - 1, 0)];
        for date in valid_dates {
            let loads = loads_data
                .loads(&vehicle_journey_idx, date)
                .unwrap_or_else(|| default_loads.as_slice());
            load_patterns_dates
                .entry(loads)
                .or_insert_with(Vec::new)
                .push(*date);
        }

        let mut result = Vec::new();

        for (loads, dates) in load_patterns_dates.into_iter() {
            let days_pattern = self
                .days_patterns
                .get_or_insert(dates.iter(), &self.calendar);

            for (offset, tz_days_pattern) in
                self.tz_patterns
                    .fetch_or_insert(timezone, &mut self.days_patterns, &self.calendar)
            {
                let splited_days_pattern = self
                    .days_patterns
                    .intersection(&days_pattern, tz_days_pattern);

                let vehicle_data = VehicleData {
                    days_pattern: splited_days_pattern,
                    vehicle_journey_idx,
                };

                let apply_offset = |x: SecondsSinceTimezonedDayStart| -> SecondsSinceUTCDayStart {
                    x.to_utc(offset.utc_minus_local())
                };

                let insert_error = self.timetables.insert(
                    stops.clone(),
                    flows.clone(),
                    board_times.clone().map(apply_offset),
                    debark_times.clone().map(apply_offset),
                    loads.iter().cloned(),
                    (),
                    vehicle_data,
                );
                match insert_error {
                    Ok(mission) => {
                        if !result.contains(&mission) {
                            result.push(mission);
                        };
                    }
                    Err(error) => {
                        handle_vehicletimes_error(vehicle_journey, &error);
                    }
                }
            }
        }
        result
    }
}

use super::generic_timetables::VehicleTimesError;
use crate::timetables::generic_timetables::{Position, Timetable, VehicleDataTrait};
use core::cmp;
use std::collections::BTreeMap;

fn handle_vehicletimes_error(vehicle_journey: &VehicleJourney, error: &VehicleTimesError) {
    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {}  because its \
                    debark time at : \n {:?} \n\
                    is earlier than its \
                    board time upstream at : \n {:?} \n. ",
                vehicle_journey.id, downstream_stop_time, upstream_stop_time
            );
        }
        VehicleTimesError::DecreasingBoardTime(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {} because its \
                    board time at : \n {:?} \n \
                    is earlier than its \
                    board time upstream at : \n {:?} \n. ",
                vehicle_journey.id, downstream_stop_time, upstream_stop_time
            );
        }
        VehicleTimesError::DecreasingDebarkTime(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            warn!(
                "Skipping vehicle journey {}  because its \
                    debark time at : \n {:?} \n \
                    is earlier than its \
                    debark time upstream at : \n {:?} \n. ",
                vehicle_journey.id, downstream_stop_time, upstream_stop_time
            );
        }
    }
}

impl<'a> TimetablesIter<'a> for PeriodicSplitVjByTzTimetables {
    type Positions = super::iters::PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        TripsIter::new(self, mission)
    }

    type Missions = super::iters::TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripsIter<'a> {
    periodic: &'a PeriodicSplitVjByTzTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: super::iters::VehicleIter,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicSplitVjByTzTimetables,
        timetable: &super::generic_timetables::Timetable,
    ) -> Self {
        let mut vehicles_iter = periodic.timetables.vehicles(timetable);
        let has_current_vehicle = vehicles_iter.next();
        let current_vehicle_days = has_current_vehicle.map(|vehicle| {
            let days_pattern = periodic.timetables.vehicle_data(&vehicle).days_pattern;
            let days_iter = periodic.days_patterns.days_in_pattern(&days_pattern);
            (vehicle, days_iter)
        });

        Self {
            periodic,
            current_vehicle_days,
            vehicles_iter,
        }
    }
}

impl<'a> Iterator for TripsIter<'a> {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((vehicle, days_iter)) = &mut self.current_vehicle_days {
                match days_iter.next() {
                    Some(day) => {
                        let trip = Trip {
                            vehicle: vehicle.clone(),
                            day,
                        };
                        return Some(trip);
                    }
                    None => {
                        let has_current_vehicle = self.vehicles_iter.next();
                        self.current_vehicle_days = has_current_vehicle.map(|vehicle| {
                            let days_pattern =
                                self.periodic.timetables.vehicle_data(&vehicle).days_pattern;
                            let days_iter =
                                self.periodic.days_patterns.days_in_pattern(&days_pattern);
                            (vehicle, days_iter)
                        });
                    }
                }
            } else {
                return None;
            }
        }
    }
}
