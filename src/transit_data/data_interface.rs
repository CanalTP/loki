use crate::{
    loads_data::{Load, LoadsData},
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
};
use chrono::{NaiveDate, NaiveDateTime};
use transit_model::{
    objects::{StopPoint, Transfer as TransitModelTransfer, VehicleJourney},
    Model,
};
pub use typed_index_collection::Idx;

use std::fmt::Debug;

use super::TransferDurations;

pub trait TransitTypes {
    /// A location where a vehicle can be boarded into or debarked from
    type Stop: Debug + Clone + 'static;

    /// A `Mission` is an ordered sequence of `Position`
    type Mission: Debug + Clone;

    /// Identify a step along a `Mission`
    /// Identify a step along a `Mission`
    type Position: Debug + Clone;

    /// A trip of a vehicle along a `Mission`
    type Trip: Debug + Clone;

    /// Identify a foot transfer between two `Stop`s
    type Transfer: Debug + Clone + 'static;
}

pub trait Data: TransitTypes {
    /// Returns `true` if `upstream` is positioned strictly before `downstream`
    /// in `mission`.
    ///
    /// Panics if `upstream` or `downstream` does not belong to `mission`.
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool;

    /// Returns `Some(next_position)` if `next_position` is after `position` on `mission`.
    ///
    /// Returns `None` if `position` is the last on `mission`.
    ///
    /// Panics if `position` does not belong to `mission`.
    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    /// Returns `Some(previous_position)` if `previous_position` is before `position` on `mission`.
    ///
    /// Returns `None` if `position` is the first on `mission`.
    ///
    /// Panics if `position` does not belong to `mission`.
    fn previous_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    /// Returns the `Mission` that `trip` belongs to.
    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission;

    /// Returns the `Stop` at `position` in `mission`
    ///
    /// Panics if `position` does not belong to `mission`
    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop;

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows boarding at `stop_idx`
    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)>;
    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows debark at `stop_idx`
    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)>;

    // Panics if `trip` does not go through `position`
    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load);

    // Panics if `trip` does not go through `position`
    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load);

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop);
    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration;
    fn transfer_transit_model_idx(&self, transfer: &Self::Transfer) -> Idx<TransitModelTransfer>;

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>;

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>;

    fn to_naive_datetime(&self, seconds: &SecondsSinceDatasetUTCStart) -> NaiveDateTime;

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney>;
    fn stop_point_idx(&self, stop: &Self::Stop) -> Idx<StopPoint>;
    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize;

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate;

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool;

    fn calendar(&self) -> &crate::time::Calendar;

    fn stop_point_idx_to_stop(&self, stop_idx: &Idx<StopPoint>) -> Option<Self::Stop>;

    fn nb_of_trips(&self) -> usize;

    /// An upper bound on the total number of `Stop`s.
    fn nb_of_stops(&self) -> usize;

    /// Returns an usize between 0 and nb_of_stops().
    ///
    /// Returns a different value for two different `stop`s.
    fn stop_id(&self, stop: &Self::Stop) -> usize;

    /// An upper bound on the total number of `Mission`s
    fn nb_of_missions(&self) -> usize;
    /// Returns an usize between 0 and nb_of_misions()
    /// Returns a different value for two different `mission`s
    fn mission_id(&self, mission: &Self::Mission) -> usize;
}

pub trait DataIO {
    fn new(
        model: &Model,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Self;
}

pub trait DataIters<'a>: TransitTypes
where
    Self::Transfer: 'a,
    Self::Stop: 'a,
{
    /// Iterator for the `Mission`s that can be boarded at a `stop`
    /// along with the `Position` of `stop` on each `Mission`
    type MissionsAtStop: Iterator<Item = (Self::Mission, Self::Position)>;
    /// Returns all the `Mission`s that can be boarded at `stop`.
    ///
    /// Should not return twice the same `Mission`.
    fn missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop;

    /// Iterator for all `Transfer`s that can be taken at a `Stop`
    type OutgoingTransfersAtStop: Iterator<
        Item = &'a (Self::Stop, TransferDurations, Self::Transfer),
    >;
    /// Returns all `Transfer`s that can be taken at `from_stop`
    ///
    /// Should not return twice the same `Transfer`.
    fn outgoing_transfers_at(&'a self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop;

    /// Iterator for all `Transfer`s that can debark at a `Stop`
    type IncomingTransfersAtStop: Iterator<
        Item = &'a (Self::Stop, TransferDurations, Self::Transfer),
    >;
    /// Returns all `Transfer`s that can debark at `stop`
    ///
    /// Should not return twice the same `Transfer`.
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop;

    /// Iterator for all `Trip`s belonging to a `Mission`.
    type TripsOfMission: Iterator<Item = Self::Trip>;
    /// Returns all `Trip`s belonging to `mission`
    fn trips_of(&'a self, mission: &Self::Mission) -> Self::TripsOfMission;
}

pub trait DataWithIters: Data + for<'a> DataIters<'a> {}
