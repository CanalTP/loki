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

use std::{collections::BTreeMap, ops::Not};

use crate::{
    loads_data::{Load, LoadsData},
    time::days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns},
};

use super::{RemovalError, TimetablesIter, day_to_timetable::DayToTimetable, generic_timetables::{Timetables, Timetable, Vehicle}};

use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
};
use crate::transit_data::{Idx, VehicleJourney};
use chrono::NaiveDate;
use chrono_tz::Tz as TimeZone;

use crate::timetables::{
    FlowDirection, Stop, Timetables as TimetablesTrait, Types as TimetablesTypes,
};

use crate::log::warn;
#[derive(Debug)]
pub struct PeriodicTimetables {
    timetables: Timetables<SecondsSinceTimezonedDayStart, (), TimeZone, VehicleData>,
    calendar: Calendar,
    days_patterns: DaysPatterns,
    vehicle_journey_to_timetables : BTreeMap<Idx<VehicleJourney>, DayToTimetable>,
}
 
#[derive(Debug, Clone)]
struct VehicleData {
    days_pattern: DaysPattern,
    vehicle_journey_idx: Idx<VehicleJourney>,
}

#[derive(Debug, Clone)]
pub struct Trip {
    vehicle: Vehicle,
    day: DaysSinceDatasetStart,
}

impl TimetablesTypes for PeriodicTimetables {
    type Mission = super::generic_timetables::Timetable;

    type Position = super::generic_timetables::Position;

    type Trip = Trip;
}

impl TimetablesTrait for PeriodicTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        let calendar = Calendar::new(first_date, last_date);
        let nb_of_days: usize = calendar.nb_of_days().into();
        Self {
            timetables: Timetables::new(),
            calendar,
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

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, _load) = self.timetables.arrival_time(&trip.vehicle, position);
        let timetable = self.timetables.timetable_of(&trip.vehicle);
        let timezone = self.timetables.timezone_data(&timetable);
        let day = &trip.day;
        let time = self.calendar.compose(day, time_in_day, timezone);
        (time, Load::default())
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let timeload_in_day = self.timetables.debark_time(&trip.vehicle, position);
        timeload_in_day.map(|(time_in_day, _load)| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            let time = self.calendar.compose(day, time_in_day, timezone);
            (time, Load::default())
        })
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let timeload_in_day = self.timetables.board_time(&trip.vehicle, position);
        timeload_in_day.map(|(time_in_day, _load)| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            let time = self.calendar.compose(day, time_in_day, timezone);
            (time, Load::default())
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let has_earliest_and_latest_board_time =
            self.timetables.earliest_and_latest_board_time(position);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (_earliest_board_time_in_day, _latest_board_time_in_day) =
            has_earliest_and_latest_board_time?;

        let timezone = self.timetables.timezone_data(&mission);

        let decompositions = self.calendar.decompositions(
            waiting_time,
            timezone,
            SecondsSinceTimezonedDayStart::max(),
            SecondsSinceTimezonedDayStart::min(),
            // *latest_board_time_in_day,
            // *earliest_board_time_in_day,
        );
        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.earliest_filtered_vehicle_to_board(
                &waiting_time_in_day,
                &mission,
                &position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, arrival_time_in_day_at_next_stop, _load)) = has_vehicle {
                let arrival_time_at_next_stop = self.calendar.compose(
                    &waiting_day,
                    arrival_time_in_day_at_next_stop,
                    &timezone,
                );
                if let Some((_, _, best_arrival_time)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if arrival_time_at_next_stop < *best_arrival_time {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, arrival_time_at_next_stop));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, arrival_time_at_next_stop));
                }
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position.map(
            |(vehicle, day, arrival_time_at_next_stop)| {
                let trip = Trip { vehicle, day };
                (trip, arrival_time_at_next_stop, Load::default())
            },
        )
    }

    fn insert<'date, Stops, Flows, Dates, Times>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: Times,
        debark_times: Times,
        _loads_data: &LoadsData,
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
        let days_pattern = self
            .days_patterns
            .get_from_dates(valid_dates, &self.calendar);

        let mut result = Vec::new();

        let mut vj_timetables = self.vehicle_journey_to_timetables
            .entry(vehicle_journey_idx)
            .or_insert_with(||DayToTimetable::new(&self.calendar));

        if let Some(day) = vj_timetables.has_intersection_with(&days_pattern, &self.days_patterns) {
            warn!("Trying to add vehicle journey {} multiple time for day {}. Insertion skipped for all days.",
                vehicle_journey.id,
                self.calendar.to_naive_date(&day)
            );
            // TODO : ? remove from days_pattern the days in the intersection and carry on with
            //          the insertion instead of returning early ?
           return result;
        }
        
        let vehicle_data = VehicleData {
            days_pattern,
            vehicle_journey_idx,
        };
        let nb_of_positions = stops.len();
        let loads = if nb_of_positions > 0 {
            vec![(); nb_of_positions - 1]
        } else {
            vec![(); 0]
        };
        let insert_result = self.timetables.insert(
            stops,
            flows,
            board_times,
            debark_times,
            loads.into_iter(),
            *timezone,
            vehicle_data,
        );
        
        match insert_result {
            Ok(mission) => {
                if result.contains(&mission).not() {
                    result.push(mission);
                }

                vj_timetables.insert_days_pattern(&days_pattern, &mission, & mut self.days_patterns, &self.calendar)
                    .unwrap(); // unwrap should be safe here, because we check above that vj_timetables has no intersection with days_pattern

            }
            Err(error) => {
                handle_vehicletimes_error(vehicle_journey, &error);
            }
        }
        result
    }

    fn remove(
        &mut self,
        date: & chrono::NaiveDate,
        vehicle_journey_idx: Idx<VehicleJourney>,
    ) -> Result<(), RemovalError> {
        let day = self.calendar.date_to_days_since_start(date).ok_or(RemovalError::UnknownDate)?;

        let has_timetables = self.vehicle_journey_to_timetables.get_mut(&vehicle_journey_idx);
        let result = match has_timetables {

            None => { // There is no timetable with this vehicle_journey_index
                Err(RemovalError::UnknownVehicleJourney)
            },
            Some(day_to_timetable) => {        
                day_to_timetable
                    .remove(&day, &mut self.days_patterns, self.calendar())
                    .map_err(|_| RemovalError::DateInvalidForVehicleJourney)

                
            }
        };

        match result {
            Err(err) => Err(err), 
            Ok(timetable) => {
                // we remove day from the day_pattern of the vehicle
                let timetable_data = self.timetables.timetable_data_mut(&timetable);
                timetable_data
                    .update_vehicles_data(|vehicle_data| {
                        vehicle_data.vehicle_journey_idx == vehicle_journey_idx
                         && self.days_patterns.is_allowed(&vehicle_data.days_pattern, &day)
                    },
                    |vehicle_data| {
                        vehicle_data.days_pattern = 
                            self.days_patterns
                            .get_pattern_without_day(vehicle_data.days_pattern, &day, &self.calendar)
                            .unwrap();  // unwrap is safe, because this updater closure will be applied only on 
                                        // a vehicle_data on which the filter closure above returns true
                                        // hence vehicle_data.days_patten has day set
                                        // and thus the pattern without day exists
                    }); 

                // by removing a day from the day_pattern, the day_pattern may have become empty
                // in this case, we remove all vehicle with an empty day_pattern
                timetable_data.remove_vehicles(|vehicle_data| {
                        self.days_patterns.is_empty_pattern(&vehicle_data.days_pattern)
                    });

                Ok(())
            }
        }



    }
}

use super::generic_timetables::VehicleTimesError;

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

impl<'a> TimetablesIter<'a> for PeriodicTimetables {
    type Positions = super::iters::PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        TripsIter::new(&self, mission)
    }

    type Missions = super::iters::TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripsIter<'a> {
    periodic: &'a PeriodicTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: super::iters::VehicleIter,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicTimetables,
        timetable: &super::generic_timetables::Timetable,
    ) -> Self {
        let mut vehicles_iter = periodic.timetables.vehicles(&timetable);
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
