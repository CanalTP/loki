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

use std::fmt::Debug;

use crate::engine::engine_interface::{Journey, RequestDebug, RequestTypes, RequestWithIters};
use crate::engine::journeys_tree::JourneysTree;
use crate::engine::pareto_front::{ArriveFront, BoardFront, DebarkFront, WaitFront};
use tracing::trace;

pub struct MultiCriteriaRaptor<T: RequestTypes> {
    journeys_tree: JourneysTree<T>,

    wait_fronts: Vec<WaitFront<T>>, // map a `stop` to a pareto front
    new_wait_fronts: Vec<WaitFront<T>>, // map a `stop` to a pareto front
    stops_with_new_wait: Vec<T::Stop>, // list of Stops

    mission_has_new_wait: Vec<Option<T::Position>>, // map a `mission` to a position

    missions_with_new_wait: Vec<T::Mission>, // list of Missions

    debark_fronts: Vec<DebarkFront<T>>, // map a `stop` to a pareto front
    new_debark_fronts: Vec<DebarkFront<T>>, // map a `stop` to a pareto front
    stops_with_new_debark: Vec<T::Stop>, // list of Stops

    board_front: BoardFront<T>,
    new_board_front: BoardFront<T>,

    arrive_front: ArriveFront<T>,

    results: Vec<Journey<T>>,

    nb_of_rounds: usize,
}

impl<T> MultiCriteriaRaptor<T>
where
    T: RequestTypes,
    T::Criteria: Debug,
{
    pub fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            journeys_tree: JourneysTree::new(),

            wait_fronts: vec![WaitFront::<T>::new(); nb_of_stops],
            new_wait_fronts: vec![WaitFront::<T>::new(); nb_of_stops],
            stops_with_new_wait: Vec::new(),

            mission_has_new_wait: vec![None; nb_of_missions],
            missions_with_new_wait: Vec::new(),

            debark_fronts: vec![DebarkFront::<T>::new(); nb_of_stops],
            new_debark_fronts: vec![DebarkFront::<T>::new(); nb_of_stops],
            stops_with_new_debark: Vec::new(),

            board_front: BoardFront::<T>::new(),
            new_board_front: BoardFront::<T>::new(),

            arrive_front: ArriveFront::<T>::new(),

            results: Vec::new(),

            nb_of_rounds: 0,
        }
    }

    pub fn nb_of_journeys(&self) -> usize {
        self.arrive_front.len()
    }

    fn resize(&mut self, nb_of_stops: usize, nb_of_missions: usize) {
        self.wait_fronts.resize(nb_of_stops, WaitFront::<T>::new());
        self.new_wait_fronts
            .resize(nb_of_stops, WaitFront::<T>::new());
        self.debark_fronts
            .resize(nb_of_stops, DebarkFront::<T>::new());
        self.new_debark_fronts
            .resize(nb_of_stops, DebarkFront::<T>::new());

        self.mission_has_new_wait.resize(nb_of_missions, None);
    }

    pub fn compute<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
        R: RequestDebug,
    {
        self.clear();
        self.resize(pt.nb_of_stops(), pt.nb_of_missions());

        self.init_with_departures(pt);

        self.identify_missions_with_new_waits(pt);

        debug_assert!(!self.missions_with_new_wait.is_empty());

        while !self.missions_with_new_wait.is_empty() {
            let nb_new_wait: usize = self.new_wait_fronts.iter().map(|front| front.len()).sum();
            trace!(
                "Round {}, nb of missions {}, new_wait {}",
                self.nb_of_rounds,
                self.missions_with_new_wait.len(),
                nb_new_wait
            );
            trace!(
                "Tree size {}, arrived {}",
                self.tree_size(),
                self.arrive_front.len()
            );

            self.save_and_clear_new_debarks(pt);

            self.ride(pt);

            self.save_and_clear_new_waits(pt);

            self.perform_transfers_and_arrivals(pt);

            self.identify_missions_with_new_waits(pt);

            self.nb_of_rounds += 1;
        }

        self.fill_results();
    }

    fn clear(&mut self) {
        self.journeys_tree.clear();
        // TODO : check which maps/lists does indeed needs clearing after compute
        //   as some of them are cleared whithin compute()
        for front in &mut self.wait_fronts {
            front.clear();
        }
        for front in &mut self.new_wait_fronts {
            front.clear();
        }
        self.stops_with_new_wait.clear();

        for opt in &mut self.mission_has_new_wait {
            *opt = None;
        }
        self.missions_with_new_wait.clear();

        for front in &mut self.debark_fronts {
            front.clear();
        }
        for front in &mut self.new_debark_fronts {
            front.clear();
        }
        self.stops_with_new_debark.clear();

        self.board_front.clear();
        self.new_board_front.clear();

        self.arrive_front.clear();

        // we don't clear self.results so as to not release the memory
        // allocated for connections in a Journey

        self.nb_of_rounds = 0;
    }

    // fill new_waiting_fronts with journeys departures
    fn init_with_departures<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
        R: RequestDebug,
    {
        debug_assert!(self.journeys_tree.is_empty());
        debug_assert!(self
            .new_wait_fronts
            .iter()
            .all(|front| { front.is_empty() }));
        debug_assert!(self.stops_with_new_wait.is_empty());

        // TODO : check that there is at least one departure
        // TODO : check that all departure stops are distincts
        for departure in pt.departures() {
            let (stop, criteria) = pt.depart(&departure);
            let journey = self.journeys_tree.depart(&departure);
            let stop_id = pt.stop_id(&stop);

            let new_wait_front = &mut self.new_wait_fronts[stop_id];
            if new_wait_front.is_empty() {
                self.stops_with_new_wait.push(stop.clone());
            }

            new_wait_front.add(journey, criteria, pt);
            trace!("Adding departure at {}", pt.stop_name(&stop));
        }
    }

    // identify missions that can be boarded from the new waiting pathes
    // - fill `mission_has_new_waiting` and `missions_with_new_waiting`
    // - reads `stops_with_new_waiting`
    fn identify_missions_with_new_waits<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
        >,
        R: RequestDebug,
    {
        debug_assert!(self.missions_with_new_wait.is_empty());

        for stop in self.stops_with_new_wait.iter() {
            // TODO : check that the same mission is not returned twice
            trace!(
                "Identifying missions with new waits at {}",
                pt.stop_name(stop)
            );
            for (mission, position) in pt.missions_at(stop) {
                let current_mission_has_new_wait =
                    &mut self.mission_has_new_wait[pt.mission_id(&mission)];
                // trace!("   mission {}", pt.mission_name(&mission));
                match current_mission_has_new_wait {
                    Some(saved_position) => {
                        if pt.is_upstream(&position, saved_position, &mission) {
                            *saved_position = position;
                        }
                    }
                    None => {
                        *current_mission_has_new_wait = Some(position);
                        self.missions_with_new_wait.push(mission)
                    }
                }

                // let current_mission_has_new_wait =
                //     self.missions_with_new_wait.entry(mission.clone());
                // use std::collections::hash_map::Entry;
                // match current_mission_has_new_wait {
                //     Entry::Vacant(entry) => {
                //         entry.insert(position);
                //     }
                //     Entry::Occupied(mut entry) => {
                //         let saved_position = entry.get_mut();
                //         if pt.is_upstream(&position, saved_position, &mission) {
                //             *saved_position = position;
                //         }
                //     }
                // }
            }
        }
    }

    // ride all `missions_with_new_waiting`, boarding all new waiting pathes,
    // propagating theses new pathes, and perform debarkments along the way
    // - update `new_debarked_fronts` and `stops_with_new_debarked`
    // - uses `onboard_front` and `new_onboard_front` as local buffers
    // - reads `missions_with_new_waiting`, `mission_has_new_waiting`,
    //         `new_waiting_fronts`, `debarked_fronts`
    fn ride<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
        R: RequestDebug,
    {
        debug_assert!(!self.missions_with_new_wait.is_empty());
        debug_assert!(self.stops_with_new_debark.is_empty());
        debug_assert!(self
            .new_debark_fronts
            .iter()
            .all(|front| { front.is_empty() }));

        for mission in self.missions_with_new_wait.iter() {
            let mut has_position = self.mission_has_new_wait[pt.mission_id(mission)].clone();

            self.board_front.clear();

            trace!(
                "Riding {} from {:?}",
                pt.mission_name(mission),
                has_position
                    .clone()
                    .map(|pos| pt.position_name(&pos, mission))
            );

            while let Some(position) = has_position {
                let stop = pt.stop_of(&position, mission);
                let stop_id = pt.stop_id(&stop);
                // update debarked front at this stop with elements from
                //   onboard front
                {
                    let debark_front = &mut self.debark_fronts[stop_id];
                    let new_debark_front = &mut self.new_debark_fronts[stop_id];
                    let new_debark_was_empty = new_debark_front.is_empty();
                    let mut new_debark_added = false;

                    // trace!("At {} Board front : {:#?}", pt.position_name(&position, mission), &self.board_front);

                    for ((ref board, ref trip), ref board_criteria) in self.board_front.iter() {
                        let has_new_debark_criteria = pt.debark(trip, &position, board_criteria);
                        if let Some(new_debark_criteria) = has_new_debark_criteria {
                            if debark_front.dominates(&new_debark_criteria, pt) {
                                continue;
                            }
                            if new_debark_front.dominates(&new_debark_criteria, pt) {
                                continue;
                            }
                            let new_debark = self.journeys_tree.debark(board, &position);
                            debark_front.remove_elements_dominated_by(&new_debark_criteria, pt);
                            new_debark_front.add_and_remove_elements_dominated(
                                new_debark,
                                new_debark_criteria,
                                pt,
                            );
                            trace!(
                                "New debark {:?} at stop {} from trip {}, parent {:?}",
                                new_debark,
                                pt.position_name(&position, mission),
                                pt.trip_name(trip),
                                board
                            );
                            new_debark_added = true;
                        }
                    }

                    if new_debark_was_empty && new_debark_added {
                        self.stops_with_new_debark.push(stop.clone());
                    }
                }

                // we update has_stop to the next stop on the route
                has_position = pt.next_on_mission(&position, mission);

                // if there is no next stop on the route
                // there is no need to the update onboard front
                if has_position.is_none() {
                    continue;
                }

                // board and ride new waitings and put them into new_onboard_front
                {
                    self.new_board_front.clear();
                    let new_wait_front = &self.new_wait_fronts[stop_id];
                    for (ref wait, ref wait_criteria) in new_wait_front.iter() {
                        // trace!("Trying to board");
                        if let Some((trip, new_board_criteria)) =
                            pt.best_trip_to_board(&position, mission, wait_criteria)
                        {
                            // trace!("Try to board at stop {} from {:?} into mission {}", pt.position_name(&position, mission), wait, pt.mission_name(&mission));
                            // trace!("New Board : \n{:#?} \n {:#?}", trip, new_board_criteria);
                            if !pt.is_valid(&new_board_criteria) {
                                // trace!("    invalid");
                                continue;
                            }
                            if self.arrive_front.dominates(&new_board_criteria, pt) {
                                // trace!("    arrive is better");
                                continue;
                            }
                            if self.new_board_front.dominates(&new_board_criteria, pt) {
                                // trace!("    new_board_front is better");
                                continue;
                            }
                            if self.can_be_discarded(&new_board_criteria, pt) {
                                continue;
                            }

                            let new_board = self.journeys_tree.board(wait, &trip, &position);
                            trace!(
                                "    New board {:?} at stop {} into trip {}, parent {:?}",
                                new_board,
                                pt.position_name(&position, mission),
                                pt.trip_name(&trip),
                                wait
                            );
                            self.new_board_front.add_and_remove_elements_dominated(
                                (new_board, trip),
                                new_board_criteria,
                                pt,
                            );
                        }
                    }
                }

                // ride to the next stop point and update onboard
                //   pareto front along the way
                {
                    for ((board, trip), criteria) in self.board_front.iter() {
                        let new_criteria = pt.ride(trip, &position, criteria);
                        if !pt.is_valid(&new_criteria) {
                            continue;
                        }
                        if self.arrive_front.dominates(&new_criteria, pt) {
                            continue;
                        }
                        if self.new_board_front.dominates(&new_criteria, pt) {
                            continue;
                        }
                        if self.can_be_discarded(&new_criteria, pt) {
                            continue;
                        }

                        self.new_board_front
                            .add((*board, trip.clone()), new_criteria, pt);
                    }
                }
                self.board_front.replace_with(&mut self.new_board_front);
            }
        }
    }

    // tranfer `new_debarked_fronts` into `debarked_fronts`
    // - update `debarked_fronts` and clear `new_debarked_fronts`
    // - reads `stops_with_new_debarked` and `new_debarked_fronts`
    fn save_and_clear_new_debarks<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
        >,
    {
        // TODO : check that new_debarked_front[stop] is empty for all
        //     stops not in stops_with_new_debarked
        for stop in &self.stops_with_new_debark {
            let stop_id = pt.stop_id(stop);
            let debark_front = &mut self.debark_fronts[stop_id];
            let new_debark_front = &mut self.new_debark_fronts[stop_id];
            //debug_assert!(!new_debark_front.is_empty());
            for (debark, criteria) in new_debark_front.iter() {
                // we do not need to check, because
                //  - new_debarked_front is a pareto front
                //  - we added an element to new_debarked_front only if it was not dominated by debarked_front
                //  - we removed from debarked_front all elements that were dominated by an element of new_debarked_front
                //
                // TODO : add debug_assert here to check what is written above
                debark_front.add_unchecked(*debark, criteria.clone());
            }
            new_debark_front.clear();
        }
        self.stops_with_new_debark.clear();
    }

    // perform transfers and arrivals from newly debarked path
    // - update `new_waiting_fronts` and `arrived_front`
    // - reads `stops_with_new_debarked`, `new_debarked_fronts`
    //         `waiting_fronts`, `new_waiting_fronts`
    fn perform_transfers_and_arrivals<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
        R: RequestDebug,
    {
        debug_assert!(self.new_wait_fronts.iter().all(|front| front.is_empty()));
        debug_assert!(self.stops_with_new_wait.is_empty());

        for arrival in pt.arrivals() {
            let stop = pt.arrival_stop(&arrival);
            let stop_id = pt.stop_id(&stop);
            let new_debark_front = &self.new_debark_fronts[stop_id];
            for (debark, criteria) in new_debark_front.iter() {
                let arrive_criteria = pt.arrive(&arrival, criteria);
                if self.arrive_front.dominates(&arrive_criteria, pt) {
                    continue;
                }
                self.arrive_front
                    .remove_elements_dominated_by(&arrive_criteria, pt);

                self.arrive_front
                    .remove_elements_that_can_be_discarded_by(&arrive_criteria, pt);

                let arrive = self.journeys_tree.arrive(debark, &arrival);

                self.arrive_front.add(arrive, arrive_criteria, pt);
                trace!("Arrival from {}, parent {:?}", pt.stop_name(&stop), debark);
            }
        }

        // we go throught all stops with a new debarked path
        for stop in self.stops_with_new_debark.iter() {
            let stop_id = pt.stop_id(stop);
            let new_debark_front = &self.new_debark_fronts[stop_id];
            debug_assert!(!new_debark_front.is_empty());
            for (debark, criteria) in new_debark_front.iter() {
                // // we perform arrival from the `debarked` path
                // if let Some(arrived_criteria) = self.pt.journey_arrival(stop, &criteria) {
                //     let arrived = self.journeys_tree.arrive(&debarked);
                //     self.arrived_front.add(arrived, arrived_criteria, self.pt);
                // }
                // we perform all transfers from the `debarked` path
                for (arrival_stop, arrival_criteria, transfer) in pt.transfers_at(stop, criteria) {
                    let arrival_id = pt.stop_id(&arrival_stop);
                    if self.can_be_discarded(&arrival_criteria, pt) {
                        continue;
                    }
                    let wait_front = &mut self.wait_fronts[arrival_id];
                    let new_wait_front = &mut self.new_wait_fronts[arrival_id];
                    if !pt.is_valid(&arrival_criteria) {
                        continue;
                    }
                    if self.arrive_front.dominates(&arrival_criteria, pt) {
                        continue;
                    }
                    if wait_front.dominates(&arrival_criteria, pt) {
                        continue;
                    }
                    if new_wait_front.dominates(&arrival_criteria, pt) {
                        continue;
                    }

                    if new_wait_front.is_empty() {
                        self.stops_with_new_wait.push(arrival_stop.clone());
                    }

                    let waiting = self.journeys_tree.transfer(debark, &transfer);
                    wait_front.remove_elements_dominated_by(&arrival_criteria, pt);
                    new_wait_front.add_and_remove_elements_dominated(waiting, arrival_criteria, pt);
                    trace!(
                        "Transfer {:?} from {} to {}, parent {:?}",
                        waiting,
                        pt.stop_name(stop),
                        pt.stop_name(&arrival_stop),
                        debark
                    );
                }
            }
        }
    }

    fn can_be_discarded<R>(&self, partial_journey_criteria: &T::Criteria, pt: &R) -> bool
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
    {
        for (_, complete_journey_criteria) in self.arrive_front.iter() {
            if pt.can_be_discarded(partial_journey_criteria, complete_journey_criteria) {
                return true;
            }
        }
        false
    }

    // tranfer `new_waiting_fronts` into `waiting_fronts`
    // - update `waiting_fronts` and clear `new_waiting_fronts`
    // - reads `stops_with_new_waiting` and `new_waiting_fronts`
    fn save_and_clear_new_waits<R>(&mut self, pt: &R)
    where
        R: RequestWithIters<
            Position = T::Position,
            Mission = T::Mission,
            Stop = T::Stop,
            Trip = T::Trip,
            Departure = T::Departure,
            Arrival = T::Arrival,
            Criteria = T::Criteria,
            Transfer = T::Transfer,
        >,
    {
        debug_assert!(!self.stops_with_new_wait.is_empty());
        // TODO : check that new_waiting_fronts[stop] is empty for all
        //     stops not in stops_with_new_waiting

        for stop in self.stops_with_new_wait.iter() {
            let stop_id = pt.stop_id(stop);
            let wait_front = &mut self.wait_fronts[stop_id];
            let new_wait_front = &mut self.new_wait_fronts[stop_id];
            debug_assert!(!new_wait_front.is_empty());
            for (wait, criteria) in new_wait_front.iter() {
                // we do not need to check, because
                //  - `new_waiting_front` is a pareto front
                //  - we added an element to `new_waiting_front` only if it was not dominated by `waiting_front`
                //  - we removed from `waiting_front` all elements that were dominated by an element of `new_waiting_front`
                //
                // TODO : add debug_assert here to check what is written above
                wait_front.add_unchecked(*wait, criteria.clone());
            }
            new_wait_front.clear();
        }
        self.stops_with_new_wait.clear();

        for mission in self.missions_with_new_wait.iter() {
            let mission_id = pt.mission_id(mission);
            self.mission_has_new_wait[mission_id] = None;
        }
        self.missions_with_new_wait.clear();
    }

    fn fill_results(&mut self) {
        for (idx, (arrived, criteria)) in self.arrive_front.iter().enumerate() {
            if idx < self.results.len() {
                let journey_to_fill = &mut self.results[idx];
                self.journeys_tree
                    .fill_journey(arrived, criteria, journey_to_fill);
            } else {
                let new_journey = self.journeys_tree.create_journey(arrived, criteria);
                self.results.push(new_journey);
            }
        }
    }

    pub fn responses(&self) -> impl Iterator<Item = &Journey<T>> {
        let nb_of_journeys = self.nb_of_journeys();
        (0..nb_of_journeys).map(move |idx| &self.results[idx])
    }

    pub fn tree_size(&self) -> usize {
        self.journeys_tree.size()
    }

    pub fn nb_of_rounds(&self) -> usize {
        self.nb_of_rounds
    }
}
