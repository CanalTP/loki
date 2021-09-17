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

pub mod basic_comparator;

use crate::{
    engine,
    loads_data::LoadsCount,
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
    transit_data::data_interface::DataIters,
    Idx, StopPoint, VehicleJourney,
};

use crate::engine::engine_interface::{BadRequest, RequestTypes};
use crate::transit_data::data_interface::Data as DataTrait;
use transit_model::Model;

use super::generic_request::{Arrival, Arrivals, Criteria, Departure, Departures};

use crate::engine::engine_interface::Journey as PTJourney;
use crate::request::generic_request::MaximizeDepartureTimeError;
use crate::request::generic_request::MaximizeDepartureTimeError::{NoBoardTime, NoTrip};
use crate::response;
use crate::timetables::generic_timetables::VehicleDataTrait;
use std::collections::HashSet;

pub struct GenericDepartAfterRequestFiltered<'data, 'model, Data: DataTrait> {
    pub(super) transit_data: &'data Data,
    pub(super) model: &'model Model,
    pub(super) departure_datetime: SecondsSinceDatasetUTCStart,
    pub(super) departures_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) arrivals_stop_point_and_fallbrack_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) leg_arrival_penalty: PositiveDuration,
    pub(super) leg_walking_penalty: PositiveDuration,
    pub(super) max_arrival_time: SecondsSinceDatasetUTCStart,
    pub(super) max_nb_legs: u8,
    pub(super) too_late_threshold: PositiveDuration,
    pub(super) forbidden_sp_idx: HashSet<Idx<StopPoint>>,
    pub(super) allowed_sp_idx: HashSet<Idx<StopPoint>>,
    pub(super) forbidden_vj_idx: HashSet<Idx<VehicleJourney>>,
    pub(super) allowed_vj_idx: HashSet<Idx<VehicleJourney>>,
}

impl<'data, 'model, Data> GenericDepartAfterRequestFiltered<'data, 'model, Data>
where
    Data: DataTrait,
{
    pub fn new(
        model: &'model transit_model::Model,
        transit_data: &'data Data,
        request_input: &engine::engine_interface::RequestInput,
    ) -> Result<Self, BadRequest>
    where
        Self: Sized,
    {
        let departure_datetime = super::generic_request::parse_datetime(
            &request_input.datetime,
            transit_data.calendar(),
        )?;

        let departures = super::generic_request::parse_departures_filtered(
            &request_input.departures_stop_point_and_fallback_duration,
            model,
            transit_data,
            |sp_idx| {
                if let true = request_input.allowed_sp_idx.is_empty() {
                    !request_input.forbidden_sp_idx.contains(&sp_idx)
                } else {
                    request_input.allowed_sp_idx.contains(&sp_idx)
                        && !request_input.forbidden_sp_idx.contains(&sp_idx)
                }
            },
        )?;

        let arrivals: Vec<_> = super::generic_request::parse_arrivals_filtered(
            &request_input.arrivals_stop_point_and_fallback_duration,
            model,
            transit_data,
            |sp_idx| {
                if let true = request_input.allowed_sp_idx.is_empty() {
                    !request_input.forbidden_sp_idx.contains(&sp_idx)
                } else {
                    request_input.allowed_sp_idx.contains(&sp_idx)
                        && !request_input.forbidden_sp_idx.contains(&sp_idx)
                }
            },
        )?;

        let result = Self {
            transit_data,
            model,
            departure_datetime,
            departures_stop_point_and_fallback_duration: departures,
            arrivals_stop_point_and_fallbrack_duration: arrivals,
            leg_arrival_penalty: request_input.leg_arrival_penalty,
            leg_walking_penalty: request_input.leg_walking_penalty,
            max_arrival_time: departure_datetime + request_input.max_journey_duration,
            max_nb_legs: request_input.max_nb_of_legs,
            too_late_threshold: request_input.too_late_threshold,
            forbidden_sp_idx: request_input.forbidden_sp_idx.clone(),
            allowed_sp_idx: request_input.allowed_sp_idx.clone(),
            forbidden_vj_idx: request_input.forbidden_vj_idx.clone(),
            allowed_vj_idx: request_input.allowed_vj_idx.clone(),
        };

        Ok(result)
    }

    pub fn create_response<R>(
        &self,
        pt_journey: &PTJourney<R>,
    ) -> Result<response::Journey<Data>, response::JourneyError<Data>>
    where
        R: RequestTypes<
            Departure = Departure,
            Arrival = Arrival,
            Trip = Data::Trip,
            Position = Data::Position,
            Transfer = Data::Transfer,
            Criteria = Criteria,
        >,
    {
        let departure_datetime = self.departure_datetime;
        let departure_idx = pt_journey.departure_leg.departure.idx;
        let departure_fallback_duration =
            &self.departures_stop_point_and_fallback_duration[departure_idx].1;

        let first_vehicle = response::VehicleLeg {
            trip: pt_journey.departure_leg.trip.clone(),
            board_position: pt_journey.departure_leg.board_position.clone(),
            debark_position: pt_journey.departure_leg.debark_position.clone(),
        };

        let arrival_fallback_duration =
            &self.arrivals_stop_point_and_fallbrack_duration[pt_journey.arrival.idx].1;

        let connections = pt_journey.connection_legs.iter().map(|connection_leg| {
            let transfer = connection_leg.transfer.clone();
            let vehicle_leg = response::VehicleLeg {
                trip: connection_leg.trip.clone(),
                board_position: connection_leg.board_position.clone(),
                debark_position: connection_leg.debark_position.clone(),
            };
            (transfer, vehicle_leg)
        });

        let journey = response::Journey::new(
            departure_datetime,
            *departure_fallback_duration,
            first_vehicle,
            connections,
            *arrival_fallback_duration,
            pt_journey.criteria_at_arrival.loads_count.clone(),
            self.transit_data,
        )
        .map_err(response::JourneyError::BadJourney)?;
        let new_journey = self
            .maximize_departure_time(journey)
            .map_err(response::JourneyError::MaximizeDepartureTimeError)?;
        Ok(new_journey)
    }

    pub fn stop_name(&self, stop: &Data::Stop) -> String {
        super::generic_request::stop_name(stop, self.model, self.transit_data)
    }

    pub fn trip_name(&self, trip: &Data::Trip) -> String {
        super::generic_request::trip_name(trip, self.model, self.transit_data)
    }

    pub fn mission_name(&self, mission: &Data::Mission) -> String {
        super::generic_request::mission_name(mission, self.model, self.transit_data)
    }

    pub fn position_name(&self, position: &Data::Position, mission: &Data::Mission) -> String {
        super::generic_request::position_name(position, mission, self.model, self.transit_data)
    }

    pub fn leg_arrival_penalty(&self) -> PositiveDuration {
        self.leg_arrival_penalty
    }

    pub fn leg_walking_penalty(&self) -> PositiveDuration {
        self.leg_walking_penalty
    }

    fn is_valid(&self, criteria: &Criteria) -> bool {
        criteria.time <= self.max_arrival_time && criteria.nb_of_legs <= self.max_nb_legs
    }

    fn can_be_discarded(
        &self,
        partial_journey_criteria: &Criteria,
        complete_journey_criteria: &Criteria,
    ) -> bool {
        partial_journey_criteria.time >= complete_journey_criteria.time + self.too_late_threshold
    }

    fn board_and_ride(
        &self,
        position: &Data::Position,
        trip: &Data::Trip,
        waiting_criteria: &Criteria,
    ) -> Option<Criteria> {
        let has_board = self.transit_data.board_time_of(trip, position);
        if let Some(board_timeload) = has_board {
            if waiting_criteria.time > board_timeload.0 {
                return None;
            }
        } else {
            return None;
        }
        let mission = self.transit_data.mission_of(trip);
        let next_position = self.transit_data.next_on_mission(position, &mission)?;
        let (arrival_time_at_next_stop, load) =
            self.transit_data.arrival_time_of(trip, &next_position);
        let new_criteria = Criteria {
            time: arrival_time_at_next_stop,
            nb_of_legs: waiting_criteria.nb_of_legs + 1,
            fallback_duration: waiting_criteria.fallback_duration,
            transfers_duration: waiting_criteria.transfers_duration,
            loads_count: waiting_criteria.loads_count.add(load),
        };
        Some(new_criteria)
    }

    fn best_trip_to_board(
        &self,
        position: &Data::Position,
        mission: &Data::Mission,
        waiting_criteria: &Criteria,
    ) -> Option<(Data::Trip, Criteria)> {
        let waiting_time = &waiting_criteria.time;

        self.transit_data
            .earliest_filtered_trip_to_board_at(
                waiting_time,
                mission,
                position,
                |vehicle_data: &Data::VehicleData| {
                    let vj_idx = vehicle_data.get_vehicle_journey_idx();
                    if let true = self.allowed_vj_idx.is_empty() {
                        !self.forbidden_vj_idx.contains(&vj_idx)
                    } else {
                        self.allowed_vj_idx.contains(&vj_idx)
                            && !self.forbidden_vj_idx.contains(&vj_idx)
                    }
                },
            )
            .map(|(trip, arrival_time, load)| {
                let new_criteria = Criteria {
                    time: arrival_time,
                    nb_of_legs: waiting_criteria.nb_of_legs + 1,
                    fallback_duration: waiting_criteria.fallback_duration,
                    transfers_duration: waiting_criteria.transfers_duration,
                    loads_count: waiting_criteria.loads_count.add(load),
                };
                (trip, new_criteria)
            })
    }

    fn debark(
        &self,
        trip: &Data::Trip,
        position: &Data::Position,
        onboard_criteria: &Criteria,
    ) -> Option<Criteria> {
        debug_assert!({
            let arrival_time = &onboard_criteria.time;
            self.transit_data.arrival_time_of(trip, position).0 == *arrival_time
        });
        self.transit_data
            .debark_time_of(trip, position)
            .map(|(debark_time, _)| Criteria {
                time: debark_time,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
                loads_count: onboard_criteria.loads_count.clone(),
            })
    }

    fn ride(&self, trip: &Data::Trip, position: &Data::Position, criteria: &Criteria) -> Criteria {
        let mission = self.transit_data.mission_of(trip);
        let next_position = self
            .transit_data
            .next_on_mission(position, &mission)
            .unwrap();
        let (arrival_time_at_next_position, load) =
            self.transit_data.arrival_time_of(trip, &next_position);
        Criteria {
            time: arrival_time_at_next_position,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration,
            loads_count: criteria.loads_count.add(load),
        }
    }

    fn depart(&self, departure: &Departure) -> (Data::Stop, Criteria) {
        let (stop, fallback_duration) =
            &self.departures_stop_point_and_fallback_duration[departure.idx];
        let time = self.departure_datetime + *fallback_duration;
        let criteria = Criteria {
            time,
            nb_of_legs: 0,
            fallback_duration: *fallback_duration,
            transfers_duration: PositiveDuration::zero(),
            loads_count: LoadsCount::zero(),
        };
        (stop.clone(), criteria)
    }

    fn arrival_stop(&self, arrival: &Arrival) -> Data::Stop {
        self.arrivals_stop_point_and_fallbrack_duration[arrival.idx]
            .0
            .clone()
    }

    fn arrive(&self, arrival: &Arrival, criteria: &Criteria) -> Criteria {
        let arrival_duration = &self.arrivals_stop_point_and_fallbrack_duration[arrival.idx].1;
        Criteria {
            time: criteria.time + *arrival_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration + *arrival_duration,
            transfers_duration: criteria.transfers_duration,
            loads_count: criteria.loads_count.clone(),
        }
    }

    fn is_upstream(
        &self,
        upstream: &Data::Position,
        downstream: &Data::Position,
        mission: &Data::Mission,
    ) -> bool {
        self.transit_data.is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Data::Position,
        mission: &Data::Mission,
    ) -> Option<Data::Position> {
        self.transit_data.next_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Data::Trip) -> Data::Mission {
        self.transit_data.mission_of(trip)
    }

    fn stop_of(&self, position: &Data::Position, mission: &Data::Mission) -> Data::Stop {
        self.transit_data.stop_of(position, mission)
    }

    fn nb_of_stops(&self) -> usize {
        self.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Data::Stop) -> usize {
        self.transit_data.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.transit_data.nb_of_missions()
    }

    fn mission_id(&self, mission: &Data::Mission) -> usize {
        self.transit_data.mission_id(mission)
    }

    // Given a 'debark_time' + 'vehicle_leg' (ie trip + board and debark positions)
    // replace the original trip by a later trip (latest possible debark_time)
    // and return the new associated board_time
    fn _maximize_leg_board_time(
        &self,
        vehicle_leg: &mut response::VehicleLeg<Data>,
        debark_time: SecondsSinceDatasetUTCStart,
    ) -> Result<SecondsSinceDatasetUTCStart, MaximizeDepartureTimeError<Data>> {
        let board_position = &vehicle_leg.board_position;
        let debark_position = &vehicle_leg.debark_position;
        let trip = &mut vehicle_leg.trip;
        let mission = &self.transit_data.mission_of(trip);
        let (new_trip, _, _) = self
            .transit_data
            .latest_trip_that_debark_at(&debark_time, mission, debark_position)
            .ok_or_else(|| NoTrip(debark_time, mission.clone(), debark_position.clone()))?;
        *trip = new_trip;
        let board_time = self
            .transit_data
            .board_time_of(trip, board_position)
            .ok_or_else(|| NoBoardTime(trip.clone(), board_position.clone()))?
            .0;
        Ok(board_time)
    }

    // Given a 'journey' (ie arrival_time + list of tranfers and vehicle_leg)
    // return a journey with the same path and the latest possible departure
    fn maximize_departure_time(
        &self,
        mut journey: response::Journey<Data>,
    ) -> Result<response::Journey<Data>, MaximizeDepartureTimeError<Data>> {
        let last_vehicle_leg = journey
            .connections
            .last()
            .map(|(_, vehicle_leg)| vehicle_leg)
            .unwrap_or(&journey.first_vehicle);

        let mut current_time = self
            .transit_data
            .debark_time_of(&last_vehicle_leg.trip, &last_vehicle_leg.debark_position)
            .ok_or_else(|| {
                NoBoardTime(
                    last_vehicle_leg.trip.clone(),
                    last_vehicle_leg.board_position.clone(),
                )
            })?
            .0;

        for (transfer, vehicle) in journey.connections.iter_mut().rev() {
            let new_board_time = self._maximize_leg_board_time(vehicle, current_time)?;
            current_time = new_board_time;

            let transfer_duration = self.transit_data.transfer_duration(transfer);
            current_time = current_time - transfer_duration;
        }

        let vehicle = &mut journey.first_vehicle;
        let _ = self._maximize_leg_board_time(vehicle, current_time)?;

        Ok(journey)
    }
}

impl<'data, 'model, 'outer, Data> GenericDepartAfterRequestFiltered<'data, 'model, Data>
where
    Data: DataTrait + DataIters<'outer>,
    Data::Transfer: 'outer,
    Data::Stop: 'outer,
{
    pub(super) fn departures(&self) -> Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_departures,
        }
    }

    pub(super) fn arrivals(&self) -> Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals {
            inner: 0..nb_of_arrivals,
        }
    }

    fn boardable_missions_at(&'outer self, stop: &Data::Stop) -> Data::MissionsAtStop {
        self.transit_data.boardable_missions_at(stop)
    }

    fn transfers_at(
        &'outer self,
        from_stop: &Data::Stop,
        criteria: &Criteria,
    ) -> TransferAtStop<'data, 'outer, Data> {
        let outgoing_transfers = self.transit_data.outgoing_transfers_at(from_stop);
        TransferAtStop {
            from_stop: from_stop.clone(),
            data: self.transit_data,
            forbidden: &self.forbidden_sp_idx,
            allowed: &self.allowed_sp_idx,
            inner: outgoing_transfers,
            criteria: criteria.clone(),
        }
    }

    fn trips_of(&'outer self, mission: &Data::Mission) -> Data::TripsOfMission {
        self.transit_data.trips_of(mission)
    }
}

pub struct TransferAtStop<'data, 'outer, Data>
where
    Data: DataTrait + DataIters<'outer>,
{
    from_stop: Data::Stop,
    data: &'data Data,
    forbidden: &'outer HashSet<Idx<StopPoint>>,
    allowed: &'outer HashSet<Idx<StopPoint>>,
    inner: Data::OutgoingTransfersAtStop,
    criteria: Criteria,
}

impl<'data, 'outer, Data> Iterator for TransferAtStop<'data, 'outer, Data>
where
    Data: DataTrait + DataIters<'outer>,
{
    type Item = (Data::Stop, Criteria, Data::Transfer);

    fn next(&mut self) -> Option<Self::Item> {
        let forbidden = self.forbidden;
        let allowed = self.allowed;
        let from_stop = &self.from_stop;
        let data = self.data;

        self.inner
            .by_ref()
            .find(|(to_stop, _, _)| {
                let from_sp_idx = data.stop_point_idx(from_stop);
                let to_sp_idx = data.stop_point_idx(to_stop);
                if let true = allowed.is_empty() {
                    !forbidden.contains(&from_sp_idx) && !forbidden.contains(&to_sp_idx)
                } else {
                    allowed.contains(&from_sp_idx)
                        && allowed.contains(&to_sp_idx)
                        && !forbidden.contains(&from_sp_idx)
                        && !forbidden.contains(&to_sp_idx)
                }
            })
            .map(|(stop, durations, transfer)| {
                let new_criteria = Criteria {
                    time: self.criteria.time + durations.total_duration,
                    nb_of_legs: self.criteria.nb_of_legs,
                    fallback_duration: self.criteria.fallback_duration,
                    transfers_duration: self.criteria.transfers_duration
                        + durations.walking_duration,
                    loads_count: self.criteria.loads_count.clone(),
                };
                (stop.clone(), new_criteria, transfer.clone())
            })
    }
}
