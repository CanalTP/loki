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

use crate::engine::engine_interface::{
    BadRequest, Request as RequestTrait, RequestDebug, RequestIO, RequestInput, RequestIters,
    RequestTypes, RequestWithIters,
};
use crate::transit_data::data_interface::TransitTypes;
use crate::transit_data::data_interface::{Data as DataTrait, DataIters, DataWithIters};

use super::{Arrival, Arrivals, Criteria, Departure, Departures, GenericDepartAfterRequest};
pub struct Request<'data, 'model, Data: DataTrait> {
    generic: GenericDepartAfterRequest<'data, 'model, Data>,
}

impl<'data, 'model, Data: DataTrait> TransitTypes for Request<'data, 'model, Data> {
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Position = Data::Position;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
}

impl<'data, 'model, Data: DataTrait> RequestTypes for Request<'data, 'model, Data> {
    type Departure = Departure;
    type Arrival = Arrival;
    type Criteria = Criteria;
}

impl<'data, 'model, Data: DataTrait> RequestTrait for Request<'data, 'model, Data> {
    fn is_lower(&self, lower: &Self::Criteria, upper: &Self::Criteria) -> bool {
        let arrival_penalty = self.generic.leg_arrival_penalty();
        let walking_penalty = self.generic.leg_walking_penalty();
        lower.time + arrival_penalty * (lower.nb_of_legs as u32)
            <= upper.time + arrival_penalty * (upper.nb_of_legs as u32)
        // && lower.nb_of_transfers <= upper.nb_of_transfers
        &&
        lower.fallback_duration + lower.transfers_duration  + walking_penalty * (lower.nb_of_legs as u32)
            <=  upper.fallback_duration + upper.transfers_duration + walking_penalty * (upper.nb_of_legs as u32)
    }

    fn can_be_discarded(
        &self,
        partial_journey_criteria: &Self::Criteria,
        complete_journey_criteria: &Self::Criteria,
    ) -> bool {
        self.generic
            .can_be_discarded(partial_journey_criteria, complete_journey_criteria)
    }

    fn is_valid(&self, criteria: &Self::Criteria) -> bool {
        self.generic.is_valid(criteria)
    }

    fn board_and_ride(
        &self,
        position: &Self::Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        self.generic
            .board_and_ride(position, trip, waiting_criteria)
    }

    fn best_trip_to_board(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
        waiting_criteria: &Self::Criteria,
    ) -> Option<(Self::Trip, Self::Criteria)> {
        self.generic
            .best_trip_to_board(position, mission, waiting_criteria)
    }

    fn debark(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        onboard_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        self.generic.debark(trip, position, onboard_criteria)
    }

    fn ride(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        criteria: &Self::Criteria,
    ) -> Self::Criteria {
        self.generic.ride(trip, position, criteria)
    }

    fn depart(&self, departure: &Self::Departure) -> (Self::Stop, Self::Criteria) {
        self.generic.depart(departure)
    }

    fn arrive(&self, arrival: &Self::Arrival, criteria: &Self::Criteria) -> Self::Criteria {
        self.generic.arrive(arrival, criteria)
    }

    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop {
        self.generic.arrival_stop(arrival)
    }

    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.generic.is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.generic.next_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.generic.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.generic.stop_of(position, mission)
    }

    fn nb_of_stops(&self) -> usize {
        self.generic.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.generic.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.generic.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.generic.mission_id(mission)
    }
}

impl<'data, 'model, 'outer, Data> RequestIters<'outer> for Request<'data, 'model, Data>
where
    Data: DataTrait + DataIters<'outer>,
    Data::Transfer: 'outer,
    Data::Stop: 'outer,
{
    type Departures = Departures;
    fn departures(&'outer self) -> Self::Departures {
        self.generic.departures()
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        self.generic.arrivals()
    }

    type MissionsAtStop = Data::MissionsAtStop;
    fn missions_at(&'outer self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.generic.missions_at(stop)
    }

    type TransfersAtStop = super::TransferAtStop<'outer, Data>;
    fn transfers_at(
        &'outer self,
        from_stop: &Self::Stop,
        criteria: &Self::Criteria,
    ) -> Self::TransfersAtStop {
        self.generic.transfers_at(from_stop, criteria)
    }

    type TripsOfMission = Data::TripsOfMission;
    fn trips_of(&'outer self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.generic.trips_of(mission)
    }
}

impl<'data, 'model, Data> RequestWithIters for Request<'data, 'model, Data> where Data: DataWithIters
{}

use crate::engine::engine_interface::Journey as PTJourney;
use crate::response;

impl<'data, 'model, Data> RequestIO<'data, 'model, Data> for Request<'data, 'model, Data>
where
    Data: DataTrait,
{
    fn new(
        model: &'model transit_model::Model,
        transit_data: &'data Data,
        request_input: &RequestInput,
    ) -> Result<Self, BadRequest>
    where
        Self: Sized,
    {
        let generic_result = GenericDepartAfterRequest::new(model, transit_data, request_input);
        generic_result.map(|generic| Self { generic })
    }

    fn data(&self) -> &Data {
        self.generic.transit_data
    }

    fn create_response<T>(
        &self,
        pt_journey: &PTJourney<T>,
    ) -> Result<response::Journey<Data>, response::JourneyError<Data>>
    where
        Self: Sized,
        T: RequestTypes<
            Stop = Self::Stop,
            Mission = Self::Mission,
            Position = Self::Position,
            Trip = Self::Trip,
            Transfer = Self::Transfer,
            Arrival = Self::Arrival,
            Departure = Self::Departure,
            Criteria = Self::Criteria,
        >,
    {
        self.generic.create_response(pt_journey)
    }
}

impl<'data, 'model, Data> RequestDebug for Request<'data, 'model, Data>
where
    Data: DataTrait,
{
    fn stop_name(&self, stop: &Self::Stop) -> String {
        self.generic.stop_name(stop)
    }

    fn trip_name(&self, trip: &Self::Trip) -> String {
        self.generic.trip_name(trip)
    }

    fn mission_name(&self, mission: &Self::Mission) -> String {
        self.generic.mission_name(mission)
    }

    fn position_name(&self, position: &Self::Position, mission: &Self::Mission) -> String {
        self.generic.position_name(position, mission)
    }
}
