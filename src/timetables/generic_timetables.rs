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

use std::{
    borrow::Borrow,
    cmp::{max, min, Ordering},
    collections::BTreeMap,
    fmt::Debug,
    ops::Not,
};
use FlowDirection::{BoardAndDebark, BoardOnly, DebarkOnly, NoBoardDebark};

use crate::timetables::{FlowDirection, Stop, StopFlows};

#[derive(Debug)]
pub(super) struct Timetables<Time, Load, TimezoneData, VehicleData> {
    pub(super) stop_flows_to_timetables: BTreeMap<StopFlows, Vec<Timetable>>,
    pub(super) timetable_datas: Vec<TimetableData<Time, Load, TimezoneData, VehicleData>>,
}

#[derive(Debug)]
pub(super) struct TimetableData<Time, Load, TimezoneData, VehicleData> {
    pub(super) timezone_data: TimezoneData,

    pub(super) stop_flows: StopFlows,

    /// vehicle data, ordered by increasing times
    /// meaning that if vehicle_1 is before vehicle_2 in this vector,
    /// then for all `position` we have
    ///    debark_times_by_position[position][vehicle_1] <= debark_times_by_position[position][vehicle_2]
    pub(super) vehicle_datas: Vec<VehicleData>,

    /// `vehicle_loads[vehicle][position]` is the load in vehicle
    /// between `position` and `position +1`
    pub(super) vehicle_loads: Vec<Vec<Load>>,

    /// `board_times_by_position[position][vehicle]`
    ///   is the time at which a traveler waiting
    ///   at `position` can board `vehicle`
    /// Vehicles are ordered by increasing time
    ///  so for each `position` the vector
    ///  board_times_by_position[position] is sorted by increasing times
    pub(super) board_times_by_position: Vec<Vec<Time>>,

    /// `debark_times_by_position[position][vehicle]`
    ///   is the time at which a traveler being inside `vehicle`
    ///   will debark at `position`
    /// Vehicles are ordered by increasing time
    ///  so for each `position` the vector
    ///  debark_times_by_position[position] is sorted by increasing times
    pub(super) debark_times_by_position: Vec<Vec<Time>>,

    pub(super) earliest_and_latest_board_time_by_position: Vec<(Time, Time)>,

    pub(super) earliest_and_latest_debark_time_by_position: Vec<(Time, Time)>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Timetable {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub(super) timetable: Timetable,
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    pub(super) timetable: Timetable,
    pub(super) idx: usize,
}

impl<Time, Load, TimezoneData, VehicleData> Timetables<Time, Load, TimezoneData, VehicleData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Clone + Debug,
    TimezoneData: PartialEq + Clone,
{
    pub(super) fn new() -> Self {
        Self {
            stop_flows_to_timetables: BTreeMap::new(),
            timetable_datas: Vec::new(),
        }
    }

    pub(super) fn timezone_data(&self, timetable: &Timetable) -> &TimezoneData {
        &self.timetable_data(timetable).timezone_data
    }

    pub(super) fn nb_of_timetables(&self) -> usize {
        self.timetable_datas.len()
    }

    pub(super) fn timetable_data(
        &self,
        timetable: &Timetable,
    ) -> &TimetableData<Time, Load, TimezoneData, VehicleData> {
        &self.timetable_datas[timetable.idx]
    }

    pub(super) fn timetable_data_mut(
        &mut self,
        timetable: &Timetable,
    ) -> &mut TimetableData<Time, Load, TimezoneData, VehicleData> {
        &mut self.timetable_datas[timetable.idx]
    }

    pub(super) fn vehicle_data(&self, vehicle: &Vehicle) -> &VehicleData {
        self.timetable_data(&vehicle.timetable)
            .vehicle_data(vehicle.idx)
    }

    pub(super) fn stoptime_idx(&self, position: &Position) -> usize {
        position.idx
    }

    pub(super) fn timetable_of(&self, vehicle: &Vehicle) -> Timetable {
        vehicle.timetable.clone()
    }

    pub(super) fn stop_at(&self, position: &Position, timetable: &Timetable) -> &Stop {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable).stop_at(position.idx)
    }

    pub(super) fn is_upstream(
        &self,
        upstream: &Position,
        downstream: &Position,
        timetable: &Timetable,
    ) -> bool {
        assert!(upstream.timetable == *timetable);
        upstream.idx < downstream.idx
    }

    pub(super) fn next_position(
        &self,
        position: &Position,
        timetable: &Timetable,
    ) -> Option<Position> {
        assert!(position.timetable == *timetable);
        if position.idx + 1 < self.timetable_data(&position.timetable).nb_of_positions() {
            let result = Position {
                timetable: position.timetable.clone(),
                idx: position.idx + 1,
            };
            Some(result)
        } else {
            None
        }
    }

    pub(super) fn previous_position(
        &self,
        position: &Position,
        timetable: &Timetable,
    ) -> Option<Position> {
        assert_eq!(position.timetable, *timetable);
        if position.idx >= 1 {
            let result = Position {
                timetable: position.timetable.clone(),
                idx: position.idx - 1,
            };
            Some(result)
        } else {
            None
        }
    }

    pub(super) fn debark_time(
        &self,
        vehicle: &Vehicle,
        position: &Position,
    ) -> Option<(&Time, &Load)> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        let time = timetable_data.debark_time(vehicle.idx, position.idx)?;
        let load = timetable_data.load_before(vehicle.idx, position.idx);
        Some((time, load))
    }

    pub(super) fn board_time(
        &self,
        vehicle: &Vehicle,
        position: &Position,
    ) -> Option<(&Time, &Load)> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        let time = timetable_data.board_time(vehicle.idx, position.idx)?;
        let load = timetable_data.load_after(vehicle.idx, position.idx);
        Some((time, load))
    }

    pub(super) fn arrival_time(&self, vehicle: &Vehicle, position: &Position) -> (&Time, &Load) {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        let time = timetable_data.arrival_time(vehicle.idx, position.idx);
        let load = timetable_data.load_before(vehicle.idx, position.idx);
        (time, load)
    }

    pub(super) fn departure_time(&self, vehicle: &Vehicle, position: &Position) -> (&Time, &Load) {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        let time = timetable_data.departure_time(vehicle.idx, position.idx);
        let load = timetable_data.load_after(vehicle.idx, position.idx);
        (time, load)
    }

    pub(super) fn earliest_and_latest_board_time(
        &self,
        position: &Position,
    ) -> Option<&(Time, Time)> {
        let timetable_data = self.timetable_data(&position.timetable);
        timetable_data.earliest_and_latest_board_time(position.idx)
    }

    pub(super) fn earliest_and_latest_debark_time(
        &self,
        position: &Position,
    ) -> Option<&(Time, Time)> {
        let timetable_data = self.timetable_data(&position.timetable);
        timetable_data.earliest_and_latest_debark_time(position.idx)
    }

    pub(super) fn earliest_vehicle_to_board(
        &self,
        waiting_time: &Time,
        timetable: &Timetable,
        position: &Position,
    ) -> Option<(Vehicle, &Time, &Load)> {
        self.earliest_filtered_vehicle_to_board(
            waiting_time,
            timetable,
            position,
            |_: &VehicleData| true,
        )
    }

    pub(super) fn earliest_filtered_vehicle_to_board<Filter>(
        &self,
        waiting_time: &Time,
        timetable: &Timetable,
        position: &Position,
        filter: Filter,
    ) -> Option<(Vehicle, &Time, &Load)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        assert!(position.timetable == *timetable);
        self.timetable_data(timetable)
            .earliest_filtered_vehicle_to_board(waiting_time, position.idx, filter)
            .map(|(idx, time)| {
                let vehicle = Vehicle {
                    timetable: timetable.clone(),
                    idx,
                };
                let load = self.timetable_data(timetable).load_after(idx, position.idx);
                (vehicle, time, load)
            })
    }

    pub(super) fn latest_vehicle_that_debark(
        &self,
        time: &Time,
        timetable: &Timetable,
        position: &Position,
    ) -> Option<(Vehicle, &Time, &Load)> {
        self.latest_filtered_vehicle_that_debark(time, timetable, position, |_| true)
    }

    pub(super) fn latest_filtered_vehicle_that_debark<Filter>(
        &self,
        time: &Time,
        timetable: &Timetable,
        position: &Position,
        filter: Filter,
    ) -> Option<(Vehicle, &Time, &Load)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        assert_eq!(position.timetable, *timetable);
        self.timetable_data(timetable)
            .latest_filtered_vehicle_that_debark(time, position.idx, filter)
            .map(|(idx, time)| {
                let vehicle = Vehicle {
                    timetable: timetable.clone(),
                    idx,
                };
                let load = self
                    .timetable_data(timetable)
                    .load_before(idx, position.idx);
                (vehicle, time, load)
            })
    }

    pub fn nb_of_trips(&self) -> usize {
        self.timetable_datas
            .iter()
            .map(|timetable| timetable.nb_of_vehicle())
            .sum()
    }

    // Insert in the trip in a timetable if
    // the given debark_times, board_times and loads are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<BoardTimes, DebarkTimes, Loads, Stops, Flows>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        timezone_data: TimezoneData,
        vehicle_data: VehicleData,
    ) -> Result<Timetable, VehicleTimesError>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Time: Clone,
        VehicleData: Clone,
    {
        let nb_of_positions = stops.len();
        assert!(nb_of_positions == flows.len());
        assert!(nb_of_positions == board_times.len());
        assert!(nb_of_positions == debark_times.len());
        assert!(nb_of_positions == loads.len() + 1);
        inspect(flows.clone(), board_times.clone(), debark_times.clone())?;

        let corrected_flows = flows.enumerate().map(|(position_idx, flow)| {
            if position_idx == 0 {
                match flow {
                    BoardAndDebark => BoardOnly,
                    DebarkOnly => NoBoardDebark,
                    _ => flow,
                }
            } else if position_idx == nb_of_positions - 1 {
                match flow {
                    BoardAndDebark => DebarkOnly,
                    BoardOnly => NoBoardDebark,
                    _ => flow,
                }
            } else {
                flow
            }
        });

        let corrected_board_debark_times = board_times
            .zip(debark_times)
            .zip(corrected_flows.clone())
            .map(
                |((board_time, debark_time), flow_direction)| match flow_direction {
                    BoardOnly => (board_time.clone(), board_time),
                    DebarkOnly => (debark_time.clone(), debark_time),
                    BoardAndDebark | NoBoardDebark => (board_time, debark_time),
                },
            );
        let corrected_board_times = corrected_board_debark_times.clone().map(|(board, _)| board);
        let corrected_debark_times = corrected_board_debark_times.map(|(_, debark)| debark);
        let stop_flows: Vec<(Stop, FlowDirection)> = stops.zip(corrected_flows).collect();
        let stop_flows_timetables = self
            .stop_flows_to_timetables
            .entry(stop_flows.clone())
            .or_insert_with(Vec::new);

        for timetable in stop_flows_timetables.iter() {
            let timetable_data = &mut self.timetable_datas[timetable.idx];
            let is_inserted = timetable_data.try_insert(
                corrected_board_times.clone(),
                corrected_debark_times.clone(),
                loads.clone(),
                timezone_data.clone(),
                vehicle_data.clone(),
            );
            if is_inserted {
                return Ok(timetable.clone());
            }
        }
        let new_timetable_data = TimetableData::new(
            stop_flows,
            corrected_board_times,
            corrected_debark_times,
            loads,
            timezone_data,
            vehicle_data,
        );
        let timetable = Timetable {
            idx: self.timetable_datas.len(),
        };
        self.timetable_datas.push(new_timetable_data);
        stop_flows_timetables.push(timetable.clone());
        Ok(timetable)
    }
}

impl<Time, Load, TimezoneData, VehicleData> TimetableData<Time, Load, TimezoneData, VehicleData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Debug,
    TimezoneData: PartialEq,
{
    fn can_board(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            BoardAndDebark | BoardOnly => true,
            NoBoardDebark | DebarkOnly => false,
        }
    }

    fn can_debark(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            BoardAndDebark | DebarkOnly => true,
            NoBoardDebark | BoardOnly => false,
        }
    }

    fn arrival_time(&self, vehicle_idx: usize, position_idx: usize) -> &Time {
        &self.debark_times_by_position[position_idx][vehicle_idx]
    }

    fn departure_time(&self, vehicle_idx: usize, position_idx: usize) -> &Time {
        &self.board_times_by_position[position_idx][vehicle_idx]
    }

    fn debark_time(&self, vehicle_idx: usize, position_idx: usize) -> Option<&Time> {
        if self.can_debark(position_idx) {
            Some(&self.debark_times_by_position[position_idx][vehicle_idx])
        } else {
            None
        }
    }

    fn board_time(&self, vehicle_idx: usize, position_idx: usize) -> Option<&Time> {
        if self.can_board(position_idx) {
            Some(&self.board_times_by_position[position_idx][vehicle_idx])
        } else {
            None
        }
    }

    fn earliest_and_latest_board_time(&self, position_idx: usize) -> Option<&(Time, Time)> {
        if self.can_board(position_idx) {
            Some(&self.earliest_and_latest_board_time_by_position[position_idx])
        } else {
            None
        }
    }

    fn earliest_and_latest_debark_time(&self, position_idx: usize) -> Option<&(Time, Time)> {
        if self.can_debark(position_idx) {
            Some(&self.earliest_and_latest_debark_time_by_position[position_idx])
        } else {
            None
        }
    }

    fn load_after(&self, vehicle_idx: usize, position_idx: usize) -> &Load {
        assert!(position_idx + 1 < self.nb_of_positions());
        &self.vehicle_loads[vehicle_idx][position_idx]
    }

    fn load_before(&self, vehicle_idx: usize, position_idx: usize) -> &Load {
        assert!(position_idx > 0);
        &self.vehicle_loads[vehicle_idx][position_idx - 1]
    }

    fn stop_at(&self, position_idx: usize) -> &Stop {
        &self.stop_flows[position_idx].0
    }

    pub(super) fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    pub(super) fn nb_of_vehicle(&self) -> usize {
        self.vehicle_datas.len()
    }

    fn vehicle_data(&self, trip_idx: usize) -> &VehicleData {
        &self.vehicle_datas[trip_idx]
    }

    // If we are waiting to board a trip at `position` at time `waiting_time`
    // return `Some(best_trip_idx)`
    // where `best_trip_idx` is the idx of the trip, among those trip on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn earliest_filtered_vehicle_to_board<Filter>(
        &self,
        waiting_time: &Time,
        position_idx: usize,
        filter: Filter,
    ) -> Option<(usize, &Time)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        if !self.can_board(position_idx) {
            return None;
        }
        let next_position_idx = position_idx + 1;
        // we should not be able to board at the last position
        assert!(next_position_idx < self.nb_of_positions());

        let last_vehicle_idx = self.board_times_by_position[position_idx].len() - 1;
        if waiting_time > &self.board_times_by_position[position_idx][last_vehicle_idx] {
            return None;
        }

        let first_boardable_vehicle = if waiting_time
            > &self.board_times_by_position[position_idx][0]
        {
            self.board_times_by_position[position_idx].partition_point(|time| time < waiting_time)
        } else {
            0
        };

        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicle() {
            let vehicle_data = &self.vehicle_datas[vehicle_idx];
            let board_time = &self.board_times_by_position[position_idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                let arrival_time_at_next_position =
                    self.arrival_time(vehicle_idx, next_position_idx);
                return Some((vehicle_idx, arrival_time_at_next_position));
            }
        }
        None
    }

    // Given a `position` and a `time`
    // return `Some(best_trip_idx)`
    // where `best_trip_idx` is the idx of the trip, among those trip on which `filter` returns true,
    // that debark at the subsequent positions at the latest time
    fn latest_filtered_vehicle_that_debark<Filter>(
        &self,
        time: &Time,
        position_idx: usize,
        filter: Filter,
    ) -> Option<(usize, &Time)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        if !self.can_debark(position_idx) {
            return None;
        }
        // we should not be able to debark at the first position
        assert!(position_idx > 0);

        if time < &self.debark_times_by_position[position_idx][0] {
            return None;
        }

        let last_vehicle_idx = self.debark_times_by_position[position_idx].len() - 1;
        let last_debarkable_vehicle =
            if time <= &self.debark_times_by_position[position_idx][last_vehicle_idx] {
                self.debark_times_by_position[position_idx].partition_point(|t| t <= time)
            } else {
                last_vehicle_idx + 1
            };

        for vehicle_idx in (0..last_debarkable_vehicle).rev() {
            let vehicle_data = &self.vehicle_datas[vehicle_idx];
            if filter(vehicle_data) {
                let departure_time_at_previous_position =
                    self.departure_time(vehicle_idx, position_idx - 1);
                return Some((vehicle_idx, departure_time_at_previous_position));
            }
        }
        None
    }

    fn new<BoardTimes, DebarkTimes, Loads>(
        stop_flows: StopFlows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        timezone_data: TimezoneData,
        vehicle_data: VehicleData,
    ) -> Self
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        let nb_of_positions = stop_flows.len();
        assert!(nb_of_positions >= 2);
        assert!(board_times.len() == nb_of_positions);
        assert!(debark_times.len() == nb_of_positions);
        assert!(loads.len() == nb_of_positions - 1);
        let earliest_and_latest_board_time_by_position: Vec<_> = board_times
            .clone()
            .map(|board_time| (board_time.clone(), board_time))
            .collect();
        let earliest_and_latest_debark_time_by_position: Vec<_> = board_times
            .clone()
            .map(|debark_time| (debark_time.clone(), debark_time))
            .collect();
        let mut result = Self {
            timezone_data,
            stop_flows,
            vehicle_datas: Vec::new(),
            vehicle_loads: Vec::new(),
            debark_times_by_position: vec![Vec::new(); nb_of_positions],
            board_times_by_position: vec![Vec::new(); nb_of_positions],
            earliest_and_latest_board_time_by_position,
            earliest_and_latest_debark_time_by_position,
        };
        result.do_insert(board_times, debark_times, loads, vehicle_data, 0);
        result
    }

    // Try to insert the trip in this timetable
    // Returns `true` if insertion was succesfull, `false` otherwise
    fn try_insert<BoardTimes, DebarkTimes, Loads>(
        &mut self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        timezone_data: TimezoneData,
        vehicle_data: VehicleData,
    ) -> bool
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        assert!(board_times.len() == self.nb_of_positions());
        assert!(debark_times.len() == self.nb_of_positions());
        assert!(loads.len() + 1 == self.nb_of_positions());
        if self.timezone_data != timezone_data {
            return false;
        }
        let has_insert_idx =
            self.find_insert_idx(board_times.clone(), debark_times.clone(), loads.clone());
        if let Some(insert_idx) = has_insert_idx {
            self.do_insert(board_times, debark_times, loads, vehicle_data, insert_idx);
            true
        } else {
            false
        }
    }

    fn find_insert_idx<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
    ) -> Option<usize>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Debug,
        Load: Debug,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        if nb_of_vehicle == 0 {
            return Some(0);
        }

        let first_board_time = board_times.clone().next().unwrap();
        let first_board_time_binary_search =
            (&self.board_times_by_position[0]).binary_search(&first_board_time);
        match first_board_time_binary_search {
            // here, first_board_time has not been found in &self.board_times_by_position[0]
            // and insert_idx is the index where this first_board_time should be inserted
            // so as to keep &self.board_times_by_position[0] sorted
            // so we  have
            //  first_board_time < &self.board_times_by_position[0][insert_idx]     if insert_idx < len
            //  first_board_time > &self.board_times_by_position[0][insert_idx -1]  if insert_idx > 0
            // so we are be able to insert the vehicle at insert_idx only if
            //       (board, debark, loads) <= vehicle_board_debark_loads(insert_idx) if insert_idx < len
            // and   (board, debark, loads) >= vehicle_board_debark_loads(insert_idx - 1) if insert_idx > 0
            Err(insert_idx) => {
                if insert_idx < self.nb_of_vehicle() {
                    match self.partial_cmp_with_vehicle(
                        board_times.clone(),
                        debark_times.clone(),
                        loads.clone(),
                        insert_idx,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Greater) => {
                            unreachable!();
                        }
                        Some(Ordering::Less) => (),
                    }
                }

                if insert_idx > 0 {
                    match self.partial_cmp_with_vehicle(
                        board_times,
                        debark_times,
                        loads,
                        insert_idx - 1,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Less) => {
                            unreachable!();
                        }
                        Some(Ordering::Greater) => (),
                    }
                }

                Some(insert_idx)
            }
            Ok(insert_idx) => {
                assert!(self.board_times_by_position[0][insert_idx] == first_board_time);
                let mut refined_insert_idx = insert_idx;
                while refined_insert_idx > 0
                    && self.board_times_by_position[0][refined_insert_idx] == first_board_time
                {
                    refined_insert_idx -= 1;
                }
                if refined_insert_idx > 0 {
                    match self.partial_cmp_with_vehicle(
                        board_times.clone(),
                        debark_times.clone(),
                        loads.clone(),
                        refined_insert_idx - 1,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Less) => {
                            unreachable!();
                        }
                        Some(Ordering::Greater) => (),
                    }
                }
                self.find_insert_idx_after(board_times, debark_times, loads, refined_insert_idx)
            }
        }
    }

    fn find_insert_idx_after<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        start_search_idx: usize,
    ) -> Option<usize>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        assert!(start_search_idx < nb_of_vehicle);

        let first_vehicle_idx = start_search_idx;
        let has_first_vehicle_comp = self.partial_cmp_with_vehicle(
            board_times.clone(),
            debark_times.clone(),
            loads.clone(),
            first_vehicle_idx,
        );

        // if the candidate is not comparable with first_vehicle
        // then we cannot add the candidate to this timetable
        let first_vehicle_comp = has_first_vehicle_comp?;
        // if first_vehicle >= candidate ,
        // then we should insert the candidate at the first position
        if first_vehicle_comp == Ordering::Less || first_vehicle_comp == Ordering::Equal {
            return Some(first_vehicle_idx);
        }
        assert!(first_vehicle_comp == Ordering::Greater);
        // otherwise, we look for a trip such that
        // prev_vehicle <= candidate <= vehicle
        let second_vehicle_idx = first_vehicle_idx + 1;
        for vehicle_idx in second_vehicle_idx..nb_of_vehicle {
            let has_vehicle_comp = self.partial_cmp_with_vehicle(
                board_times.clone(),
                debark_times.clone(),
                loads.clone(),
                vehicle_idx,
            );
            // if the candidate is not comparable with vehicle
            // then we cannot add the candidate to this timetable
            let vehicle_cmp = has_vehicle_comp?;

            if vehicle_cmp == Ordering::Less || vehicle_cmp == Ordering::Equal {
                return Some(vehicle_idx);
            }
            assert!(vehicle_cmp == Ordering::Greater);
        }

        // here  candidate_  >= vehicle for all vehicles,
        // so we can insert the candidate as the last vehicle
        Some(nb_of_vehicle)
    }

    fn do_insert<BoardTimes, DebarkTimes, Loads>(
        &mut self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_data: VehicleData,
        insert_idx: usize,
    ) where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
    {
        if insert_idx < self.nb_of_vehicle() {
            assert!({
                let insert_cmp = self.partial_cmp_with_vehicle(
                    board_times.clone(),
                    debark_times.clone(),
                    loads.clone(),
                    insert_idx,
                );
                insert_cmp == Some(Ordering::Less) || insert_cmp == Some(Ordering::Equal)
            });
        }
        if insert_idx > 0 {
            assert!({
                let prev_insert_cmp = self.partial_cmp_with_vehicle(
                    board_times.clone(),
                    debark_times.clone(),
                    loads.clone(),
                    insert_idx - 1,
                );
                prev_insert_cmp == Some(Ordering::Greater)
            });
        }

        for (position, (board_time, debark_time)) in board_times.zip(debark_times).enumerate() {
            self.board_times_by_position[position].insert(insert_idx, board_time.clone());
            self.debark_times_by_position[position].insert(insert_idx, debark_time);

            let (earliest, latest) = &mut self.earliest_and_latest_board_time_by_position[position];
            *earliest = min(earliest.clone(), board_time.clone());
            *latest = max(latest.clone(), board_time);
        }
        self.vehicle_datas.insert(insert_idx, vehicle_data);

        let loads_vec: Vec<Load> = loads.collect();
        self.vehicle_loads.insert(insert_idx, loads_vec);
    }

    fn partial_cmp_with_vehicle<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_idx: usize,
    ) -> Option<Ordering>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        let board_cmp = partial_cmp(board_times, self.vehicle_board_times(vehicle_idx))?;
        let debark_cmp = partial_cmp(debark_times, self.vehicle_debark_times(vehicle_idx))?;

        let board_debark_cmp = combine(board_cmp, debark_cmp)?;
        let loads_cmp = partial_cmp(loads, self.vehicle_loads(vehicle_idx))?;
        combine(board_debark_cmp, loads_cmp)
    }

    pub(super) fn remove_vehicle(&mut self, vehicle_idx: usize) -> Result<(), ()> {
        if vehicle_idx >= self.nb_of_vehicle() {
            return Err(());
        }

        for board_times in self.board_times_by_position.iter_mut() {
            board_times.remove(vehicle_idx);
        }
        for debark_times in self.debark_times_by_position.iter_mut() {
            debark_times.remove(vehicle_idx);
        }

        self.vehicle_loads.remove(vehicle_idx);
        self.vehicle_datas.remove(vehicle_idx);

        Ok(())
    }

    pub(super) fn remove_vehicles<Filter>(&mut self, vehicle_filter: Filter) -> usize
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let nb_to_remove = self
            .vehicle_datas
            .iter()
            .filter(|vehicle_data| vehicle_filter(vehicle_data))
            .count();
        if nb_to_remove == 0 {
            return 0;
        }

        //  Option 1 : use buffers to copy the data to keep, and then make swaps
        //             to obtain the data to keep : iterate on the zip(vec_to_modify, vehicle_data)
        //
        //   Option 2 : use retain with a closure whose state tracks the current index/vehicle
        //              see https://stackoverflow.com/a/59602788
        for board_times in self.board_times_by_position.iter_mut() {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            board_times.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }
        for debark_times in self.debark_times_by_position.iter_mut() {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            debark_times.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }

        {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            self.vehicle_loads.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }

        {
            self.vehicle_datas
                .retain(|vehicle_data| vehicle_filter(vehicle_data).not());
        }

        nb_to_remove
    }

    pub fn update_vehicles_data<Updater>(&mut self, mut updater: Updater) -> Result<usize, ()>
    where
        Updater: FnMut(&mut VehicleData) -> bool, // returns true when an update took place
    {
        let mut nb_updated = 0usize;
        for vehicle_data in self.vehicle_datas.iter_mut() {
            let updated = updater(vehicle_data);
            if updated {
                nb_updated += 1;
            }
        }

        match nb_updated {
            0 => Err(()),
            _ => Ok(nb_updated),
        }
    }
}

fn combine(a: Ordering, b: Ordering) -> Option<Ordering> {
    use Ordering::{Equal, Greater, Less};
    match (a, b) {
        (Less, Less) | (Less, Equal) | (Equal, Less) => Some(Less),
        (Equal, Equal) => Some(Equal),
        (Greater, Greater) | (Greater, Equal) | (Equal, Greater) => Some(Greater),
        _ => None,
    }
}

// Retuns
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
pub(super) fn partial_cmp<Lower, Upper, Value, UpperVal, LowerVal>(
    lower: Lower,
    upper: Upper,
) -> Option<Ordering>
where
    Lower: Iterator<Item = UpperVal> + Clone,
    Upper: Iterator<Item = LowerVal> + Clone,
    Value: Ord,
    UpperVal: Borrow<Value>,
    LowerVal: Borrow<Value>,
{
    debug_assert!(lower.clone().count() == upper.clone().count());
    let zip_iter = lower.zip(upper);
    let mut first_not_equal_iter =
        zip_iter.skip_while(|(lower_val, upper_val)| lower_val.borrow() == upper_val.borrow());
    let has_first_not_equal = first_not_equal_iter.next();
    if let Some(first_not_equal) = has_first_not_equal {
        let ordering = {
            let lower_val = first_not_equal.0;
            let upper_val = first_not_equal.1;
            lower_val.borrow().cmp(upper_val.borrow())
        };
        debug_assert!(ordering != Ordering::Equal);
        // let's see if there is an index where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.borrow().cmp(upper_val.borrow());
            cmp != ordering && cmp != Ordering::Equal
        });
        if found.is_some() {
            return None;
        }
        // if found.is_none(), it means that
        // all elements are ordered the same, so the two vectors are comparable
        return Some(ordering);
    }
    // if has_first_not_equal == None
    // then values == item_values
    // the two vector are equal
    Some(Ordering::Equal)
}

fn is_increasing<EnumeratedValues, Value>(
    mut enumerated_values: EnumeratedValues,
) -> Result<(), (usize, usize)>
where
    EnumeratedValues: Iterator<Item = (usize, Value)>,
    Value: Ord,
{
    let has_previous = enumerated_values.next();
    let (mut prev_position, mut prev_value) = has_previous.unwrap();
    for (position, value) in enumerated_values {
        if value < prev_value {
            return Err((prev_position, position));
        }
        prev_position = position;
        prev_value = value;
    }
    Ok(())
}

pub(super) fn inspect<BoardTimes, DebarkTimes, Flows, Time>(
    flows: Flows,
    board_times: BoardTimes,
    debark_times: DebarkTimes,
) -> Result<(), VehicleTimesError>
where
    BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
    DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
    Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
    Time: Ord + Clone,
{
    assert!(flows.len() == board_times.len());
    assert!(flows.len() == debark_times.len());

    let valid_enumerated_board_times = board_times
        .clone()
        .zip(flows.clone())
        .enumerate()
        .filter_map(
            |(position, (board_time, flow_direction))| match flow_direction {
                BoardOnly | BoardAndDebark => Some((position, board_time)),
                NoBoardDebark | DebarkOnly => None,
            },
        );

    if let Err((upstream, downstream)) = is_increasing(valid_enumerated_board_times) {
        let position_pair = PositionPair {
            upstream,
            downstream,
        };
        return Err(VehicleTimesError::DecreasingBoardTime(position_pair));
    }

    let valid_enumerated_debark_times = debark_times
        .clone()
        .zip(flows.clone())
        .enumerate()
        .filter_map(
            |(position, (debark_time, flow_direction))| match flow_direction {
                DebarkOnly | BoardAndDebark => Some((position, debark_time)),
                NoBoardDebark | BoardOnly => None,
            },
        );

    if let Err((upstream, downstream)) = is_increasing(valid_enumerated_debark_times) {
        let position_pair = PositionPair {
            upstream,
            downstream,
        };
        return Err(VehicleTimesError::DecreasingDebarkTime(position_pair));
    }

    let pair_iter = board_times
        .zip(flows.clone())
        .zip(debark_times.zip(flows).skip(1))
        .enumerate();
    for (board_idx, ((board_time, board_flow), (debark_time, debark_flow))) in pair_iter {
        let debark_idx = board_idx + 1;
        let can_board = match board_flow {
            BoardAndDebark | BoardOnly => true,
            NoBoardDebark | DebarkOnly => false,
        };
        let can_debark = match debark_flow {
            BoardAndDebark | DebarkOnly => true,
            NoBoardDebark | BoardOnly => false,
        };
        if can_board && can_debark && board_time > debark_time {
            let position_pair = PositionPair {
                upstream: board_idx,
                downstream: debark_idx,
            };
            return Err(VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair));
        }
    }

    Ok(())
}

pub(super) struct PositionPair {
    pub upstream: usize,
    pub downstream: usize,
}

pub(super) enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),       // board_time[upstream] > board_time[downstream]
    DecreasingDebarkTime(PositionPair),      // debark_time[upstream] > debark_time[downstream]
}
