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

use crate::transit_data::iters::MissionsOfStop;
use crate::{
    loads_data::Load,
    time::{Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
    TransitData,
};
pub use transit_model::objects::Time as TransitModelTime;
pub use transit_model::objects::{StopPoint, Transfer as TransitModelTransfer, VehicleJourney};
pub use typed_index_collection::Idx;

use std::fmt::Debug;

use crate::timetables::generic_timetables::VehicleDataTrait;
use crate::timetables::{Stop, Timetables as TimetablesTrait, TimetablesIter};
use crate::transit_data::data_interface::Data;
use crate::transit_data::{data_interface, iters, StopData, Transfer};
use std::collections::HashSet;

#[derive(Default)]
pub struct DataFilter {
    pub forbidden_sp_idx: HashSet<Idx<StopPoint>>,
    pub allowed_sp_idx: HashSet<Idx<StopPoint>>,
    pub forbidden_vj_idx: HashSet<Idx<VehicleJourney>>,
    pub allowed_vj_idx: HashSet<Idx<VehicleJourney>>,
}

impl DataFilter {
    pub fn is_empty(&self) -> bool {
        self.forbidden_sp_idx.is_empty()
            && self.allowed_sp_idx.is_empty()
            && self.forbidden_vj_idx.is_empty()
            && self.allowed_vj_idx.is_empty()
    }
    pub fn is_sp_allowed(&self, stop_idx: Idx<StopPoint>) -> bool {
        if self.allowed_sp_idx.is_empty() {
            !self.forbidden_sp_idx.contains(&stop_idx)
        } else {
            self.allowed_sp_idx.contains(&stop_idx) && !self.forbidden_sp_idx.contains(&stop_idx)
        }
    }

    pub fn is_vj_allowed(&self, vj_idx: Idx<VehicleJourney>) -> bool {
        if self.allowed_vj_idx.is_empty() {
            !self.forbidden_vj_idx.contains(&vj_idx)
        } else {
            self.allowed_vj_idx.contains(&vj_idx) && !self.forbidden_vj_idx.contains(&vj_idx)
        }
    }
}

pub struct TransitDataFiltered<'data, Timetables: TimetablesTrait> {
    pub transit_data: &'data TransitData<Timetables>,
    pub filters: DataFilter,
}

impl<'data, Timetables: TimetablesTrait> TransitDataFiltered<'data, Timetables> {
    pub fn stop_data(&self, stop: &Stop) -> &StopData<Timetables> {
        &self.transit_data.stops_data[stop.idx]
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx: &Idx<StopPoint>) -> Option<&Stop> {
        self.transit_data.stop_point_idx_to_stop.get(stop_point_idx)
    }

    pub fn new(data: &'data TransitData<Timetables>, filters: DataFilter) -> Self {
        Self {
            transit_data: data,
            filters,
        }
    }
}

impl<Timetables: TimetablesTrait> data_interface::TransitTypes
    for TransitDataFiltered<'_, Timetables>
{
    type Stop = Stop;
    type Mission = Timetables::Mission;
    type Position = Timetables::Position;
    type Trip = Timetables::Trip;
    type Transfer = Transfer;
}

impl<Timetables: TimetablesTrait> data_interface::Data for TransitDataFiltered<'_, Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.transit_data
            .timetables
            .is_upstream_in_mission(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.transit_data
            .timetables
            .next_position(position, mission)
    }

    fn previous_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.transit_data
            .timetables
            .previous_position(position, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.transit_data.timetables.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.transit_data.timetables.stop_at(position, mission)
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let mission = self.mission_of(trip);
        let stop = self.stop_of(position, &mission);
        let stop_idx = self.stop_point_idx(&stop);

        if self.filters.is_sp_allowed(stop_idx) {
            self.transit_data.timetables.board_time_of(trip, position)
        } else {
            None
        }
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let mission = self.mission_of(trip);
        let stop = self.stop_of(position, &mission);
        let stop_idx = self.stop_point_idx(&stop);

        if self.filters.is_sp_allowed(stop_idx) {
            self.transit_data.timetables.debark_time_of(trip, position)
        } else {
            None
        }
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.transit_data.timetables.arrival_time_of(trip, position)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.transit_data
            .timetables
            .departure_time_of(trip, position)
    }

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop) {
        let transfer_data = &self.transit_data.transfers_data[transfer.idx];
        (transfer_data.from_stop, transfer_data.to_stop)
    }

    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration {
        let transfer_data = &self.transit_data.transfers_data[transfer.idx];
        transfer_data.durations.total_duration
    }

    fn transfer_transit_model_idx(&self, transfer: &Self::Transfer) -> Idx<TransitModelTransfer> {
        let transfer_data = &self.transit_data.transfers_data[transfer.idx];
        transfer_data.transit_model_transfer_idx
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let stop = self.stop_of(position, mission);
        let stop_idx = self.stop_point_idx(&stop);

        if self.filters.is_sp_allowed(stop_idx) {
            self.transit_data
                .timetables
                .earliest_filtered_trip_to_board_at(
                    waiting_time,
                    mission,
                    position,
                    |vehicle_data: &Timetables::VehicleData| {
                        let vj_idx = vehicle_data.get_vehicle_journey_idx();
                        self.filters.is_vj_allowed(vj_idx)
                    },
                )
        } else {
            None
        }
    }

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let stop = self.stop_of(position, mission);
        let stop_idx = self.stop_point_idx(&stop);

        if self.filters.is_sp_allowed(stop_idx) {
            self.transit_data
                .timetables
                .latest_filtered_trip_that_debark_at(
                    waiting_time,
                    mission,
                    position,
                    |vehicle_data: &Timetables::VehicleData| {
                        let vj_idx = vehicle_data.get_vehicle_journey_idx();
                        self.filters.is_vj_allowed(vj_idx)
                    },
                )
        } else {
            None
        }
    }

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.transit_data
            .timetables
            .calendar()
            .to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney> {
        self.transit_data.timetables.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Stop) -> Idx<StopPoint> {
        self.transit_data.stops_data[stop.idx].stop_point_idx
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize {
        self.transit_data.timetables.stoptime_idx(position, trip)
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        self.transit_data.timetables.day_of(trip)
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        stop_a.idx == stop_b.idx
    }

    fn calendar(&self) -> &Calendar {
        self.transit_data.timetables.calendar()
    }

    fn stop_point_idx_to_stop(&self, stop_point_idx: &Idx<StopPoint>) -> Option<Self::Stop> {
        self.transit_data
            .stop_point_idx_to_stop
            .get(stop_point_idx)
            .copied()
    }

    fn nb_of_trips(&self) -> usize {
        self.transit_data.timetables.nb_of_trips()
    }

    fn nb_of_stops(&self) -> usize {
        self.transit_data.stops_data.len()
    }

    fn stop_id(&self, stop: &Stop) -> usize {
        stop.idx
    }

    fn nb_of_missions(&self) -> usize {
        self.transit_data.timetables.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.transit_data.timetables.mission_id(mission)
    }
}

impl<'a, Timetables> data_interface::DataIters<'a> for TransitDataFiltered<'_, Timetables>
where
    Timetables: TimetablesTrait + for<'b> TimetablesIter<'b> + Debug,
    Timetables::Mission: 'a,
    Timetables::Position: 'a,
{
    type MissionsAtStop = MissionsOfStop<'a, Timetables>;

    fn missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.transit_data.missions_of_filtered(stop, |inner_stop| {
            let sp_idx = self.transit_data.stop_point_idx(inner_stop);
            self.filters.is_sp_allowed(sp_idx)
        })
    }

    type OutgoingTransfersAtStop = iters::OutgoingTransfersAtStop<'a>;
    fn outgoing_transfers_at(&'a self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop {
        self.transit_data.outgoing_transfers_at(from_stop)
    }

    type IncomingTransfersAtStop = iters::IncomingTransfersAtStop<'a>;
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.transit_data.incoming_transfers_at(stop)
    }

    type TripsOfMission = <Timetables as TimetablesIter<'a>>::Trips;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.transit_data.timetables.trips_of(mission)
    }
}

impl<Timetables> data_interface::DataWithIters for TransitDataFiltered<'_, Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
}
