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
pub mod loads_comparator;

use crate::{
    loads_data::LoadsCount,
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
    transit_data::data_interface::DataIters,
};

use crate::engine::engine_interface::{BadRequest, RequestInput, RequestTypes};
use crate::transit_data::data_interface::Data as DataTrait;
use transit_model::Model;

pub struct GenericArriveBeforeRequest<'data, 'model, Data: DataTrait> {
    pub(super) transit_data: &'data Data,
    pub(super) model: &'model Model,
    pub(super) arrival_datetime: SecondsSinceDatasetUTCStart,
    pub(super) entry_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) exit_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) leg_arrival_penalty: PositiveDuration,
    pub(super) leg_walking_penalty: PositiveDuration,
    pub(super) min_departure_time: SecondsSinceDatasetUTCStart,
    pub(super) max_nb_legs: u8,
    pub(super) too_late_threshold: PositiveDuration,
}

impl<'data, 'model, Data> GenericArriveBeforeRequest<'data, 'model, Data>
where
    Data: DataTrait,
{
    pub fn new(
        model: &'model transit_model::Model,
        transit_data: &'data Data,
        request_input: &RequestInput,
    ) -> Result<Self, BadRequest>
    where
        Self: Sized,
    {
        let arrival_datetime = super::generic_request::parse_datetime(
            &request_input.datetime,
            transit_data.calendar(),
        )?;

        let departures = super::generic_request::parse_departures(
            &request_input.departures_stop_point_and_fallback_duration,
            model,
            transit_data,
        )?;

        let arrivals: Vec<_> = super::generic_request::parse_arrivals(
            &request_input.arrivals_stop_point_and_fallback_duration,
            model,
            transit_data,
        )?;

        let result = Self {
            transit_data,
            model,
            arrival_datetime,
            entry_stop_point_and_fallback_duration: departures,
            exit_stop_point_and_fallback_duration: arrivals,
            leg_arrival_penalty: request_input.leg_arrival_penalty,
            leg_walking_penalty: request_input.leg_walking_penalty,
            min_departure_time: arrival_datetime - request_input.max_journey_duration,
            max_nb_legs: request_input.max_nb_of_legs,
            too_late_threshold: request_input.too_late_threshold,
        };

        Ok(result)
    }

    pub fn create_arrive_before_response<R>(
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
        let departure_datetime = pt_journey.criteria_at_arrival.time;

        let arrival_fallback_duration = {
            let departure_idx = pt_journey.departure_leg.departure.idx;
            &self.exit_stop_point_and_fallback_duration[departure_idx].1
        };

        let departure_fallback_duration = {
            let arrival_idx = pt_journey.arrival.idx;
            &self.entry_stop_point_and_fallback_duration[arrival_idx].1
        };

        let first_vehicle = if let true = pt_journey.connection_legs.is_empty() {
            response::VehicleLeg {
                trip: pt_journey.departure_leg.trip.clone(),
                board_position: pt_journey.departure_leg.debark_position.clone(),
                debark_position: pt_journey.departure_leg.board_position.clone(),
            }
        } else {
            // last connection become first vehicle && we inverse board & debark
            let last_connection = pt_journey.connection_legs.last();
            response::VehicleLeg {
                trip: last_connection.unwrap().trip.clone(),
                board_position: last_connection.unwrap().debark_position.clone(),
                debark_position: last_connection.unwrap().board_position.clone(),
            }
        };

        let transfer_iter = pt_journey
            .connection_legs
            .iter()
            .rev()
            .map(|connection_leg| connection_leg.transfer.clone());

        let time_forward_vehicle_leg_iter = pt_journey
            .connection_legs
            .iter()
            .rev()
            .skip(1)
            .map(|connection_leg| response::VehicleLeg {
                trip: connection_leg.trip.clone(),
                board_position: connection_leg.debark_position.clone(),
                debark_position: connection_leg.board_position.clone(),
            })
            .chain(std::iter::once(response::VehicleLeg {
                trip: pt_journey.departure_leg.trip.clone(),
                board_position: pt_journey.departure_leg.debark_position.clone(),
                debark_position: pt_journey.departure_leg.board_position.clone(),
            }));

        // reverse iterator
        let connections = transfer_iter.zip(time_forward_vehicle_leg_iter);

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
            .minimize_arrival_time(journey)
            .map_err(response::JourneyError::MinimizeArrivalTimeError)?;
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
        criteria.time >= self.min_departure_time && criteria.nb_of_legs <= self.max_nb_legs
    }

    fn can_be_discarded(
        &self,
        partial_journey_criteria: &Criteria,
        complete_journey_criteria: &Criteria,
    ) -> bool {
        partial_journey_criteria.time <= complete_journey_criteria.time - self.too_late_threshold
    }

    fn board_and_ride(
        &self,
        position: &Data::Position,
        trip: &Data::Trip,
        waiting_criteria: &Criteria,
    ) -> Option<Criteria> {
        let has_debark = self.transit_data.debark_time_of(trip, position);
        if let Some(debark_timeload) = has_debark {
            if waiting_criteria.time < debark_timeload.0 {
                return None;
            }
        } else {
            return None;
        }
        let mission = self.transit_data.mission_of(trip);
        let previous_position = self.transit_data.previous_on_mission(position, &mission)?;
        let (departure_time_at_previous_stop, load) = self
            .transit_data
            .departure_time_of(trip, &previous_position);
        let new_criteria = Criteria {
            time: departure_time_at_previous_stop,
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
            .latest_trip_that_debark_at(waiting_time, mission, position)
            .map(|(trip, debark_time, load)| {
                let new_criteria = Criteria {
                    time: debark_time,
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
            self.transit_data.departure_time_of(trip, position).0 == *arrival_time
        });
        self.transit_data
            .board_time_of(trip, position)
            .map(|(board_time, _load)| Criteria {
                time: board_time,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
                loads_count: onboard_criteria.loads_count.clone(),
            })
    }

    fn ride(&self, trip: &Data::Trip, position: &Data::Position, criteria: &Criteria) -> Criteria {
        let mission = self.transit_data.mission_of(trip);
        let previous_position = self
            .transit_data
            .previous_on_mission(position, &mission)
            .unwrap();
        let (departure_time_at_previous_position, load) = self
            .transit_data
            .departure_time_of(trip, &previous_position);
        Criteria {
            time: departure_time_at_previous_position,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration,
            loads_count: criteria.loads_count.add(load),
        }
    }

    fn depart(&self, departure: &Departure) -> (Data::Stop, Criteria) {
        let (stop, fallback_duration) = &self.exit_stop_point_and_fallback_duration[departure.idx];
        let time = self.arrival_datetime - *fallback_duration;
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
        self.entry_stop_point_and_fallback_duration[arrival.idx]
            .0
            .clone()
    }

    fn arrive(&self, arrival: &Arrival, criteria: &Criteria) -> Criteria {
        let arrival_duration = &self.entry_stop_point_and_fallback_duration[arrival.idx].1;
        Criteria {
            time: criteria.time - *arrival_duration,
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
        !self.transit_data.is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Data::Position,
        mission: &Data::Mission,
    ) -> Option<Data::Position> {
        self.transit_data.previous_on_mission(stop, mission)
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

    // Given a 'board_time' + 'vehicle_leg' (ie trip + board and debark positions)
    // replace the original trip by an earlier trip (earliest possible board_time)
    // and return the new associated debark_time
    fn _minimize_leg_debark_time(
        &self,
        vehicle_leg: &mut response::VehicleLeg<Data>,
        board_time: SecondsSinceDatasetUTCStart,
    ) -> Result<SecondsSinceDatasetUTCStart, MinimizeArrivalTimeError<Data>> {
        let board_position = &vehicle_leg.board_position;
        let debark_position = &vehicle_leg.debark_position;
        let trip = &mut vehicle_leg.trip;
        let mission = &self.transit_data.mission_of(trip);
        let (new_trip, _, _) = self
            .transit_data
            .earliest_trip_to_board_at(&board_time, mission, board_position)
            .ok_or_else(|| NoTrip(board_time, mission.clone(), board_position.clone()))?;
        *trip = new_trip;
        let debark_time = self
            .transit_data
            .debark_time_of(trip, debark_position)
            .ok_or_else(|| NoDebarkTime(trip.clone(), board_position.clone()))?
            .0;
        Ok(debark_time)
    }

    // Given a 'journey' (ie daparture_time + list of tranfers and vehicle_leg)
    // return a journey with the same path and the earliest possible arrival_time
    fn minimize_arrival_time(
        &self,
        mut journey: response::Journey<Data>,
    ) -> Result<response::Journey<Data>, MinimizeArrivalTimeError<Data>> {
        let vehicle = &mut journey.first_vehicle;
        let mut current_time = self
            .transit_data
            .board_time_of(&vehicle.trip, &vehicle.board_position)
            .ok_or_else(|| NoBoardTime(vehicle.trip.clone(), vehicle.board_position.clone()))?
            .0;
        let new_debark_time = self._minimize_leg_debark_time(vehicle, current_time)?;
        current_time = new_debark_time;

        for (transfer, vehicle) in journey.connections.iter_mut() {
            // increase time by transfer_duration
            let transfer_duration = self.transit_data.transfer_duration(transfer);
            current_time = current_time + transfer_duration;

            let new_debark_time = self._minimize_leg_debark_time(vehicle, current_time)?;
            current_time = new_debark_time;
        }
        Ok(journey)
    }
}

use crate::engine::engine_interface::Journey as PTJourney;
use crate::response;

use super::generic_request::{Arrival, Arrivals, Criteria, Departure, Departures};
use crate::request::generic_request::MinimizeArrivalTimeError;
use crate::request::generic_request::MinimizeArrivalTimeError::*;

impl<'data, 'model, 'outer, Data> GenericArriveBeforeRequest<'data, 'model, Data>
where
    Data: DataTrait + DataIters<'outer>,
    Data::Transfer: 'outer,
    Data::Stop: 'outer,
{
    fn departures(&'outer self) -> Departures {
        let nb_of_arrivals = self.exit_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_arrivals,
        }
    }

    fn arrivals(&'outer self) -> Arrivals {
        let nb_of_departures = self.entry_stop_point_and_fallback_duration.len();
        Arrivals {
            inner: 0..nb_of_departures,
        }
    }

    fn missions_at(&'outer self, stop: &Data::Stop) -> Data::MissionsAtStop {
        self.transit_data.missions_at(stop)
    }

    fn transfers_at(
        &'outer self,
        from_stop: &Data::Stop,
        criteria: &Criteria,
    ) -> TransferAtStop<'outer, Data> {
        let incoming_transfers = self.transit_data.incoming_transfers_at(from_stop);
        TransferAtStop {
            inner: incoming_transfers,
            criteria: criteria.clone(),
        }
    }

    fn trips_of(&'outer self, mission: &Data::Mission) -> Data::TripsOfMission {
        self.transit_data.trips_of(mission)
    }
}

pub struct TransferAtStop<'data, Data>
where
    Data: DataTrait + DataIters<'data>,
{
    inner: Data::IncomingTransfersAtStop,
    criteria: Criteria,
}

impl<'data, Data> Iterator for TransferAtStop<'data, Data>
where
    Data: DataTrait + DataIters<'data>,
{
    type Item = (Data::Stop, Criteria, Data::Transfer);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(stop, durations, transfer)| {
            let new_criteria = Criteria {
                time: self.criteria.time - durations.total_duration,
                nb_of_legs: self.criteria.nb_of_legs,
                fallback_duration: self.criteria.fallback_duration,
                transfers_duration: self.criteria.transfers_duration + durations.walking_duration,
                loads_count: self.criteria.loads_count.clone(),
            };
            (stop.clone(), new_criteria, transfer.clone())
        })
    }
}
