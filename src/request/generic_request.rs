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

use std::marker::PhantomData;

use crate::{
    loads_data::LoadsCount,
    time::{Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
    transit_data::data_interface::TransitTypes,
    Idx, RequestTypes,
};

use crate::engine::engine_interface::BadRequest;
use crate::transit_data::data_interface::Data as DataTrait;
use chrono::NaiveDateTime;
use std::fmt::Debug;
use tracing::warn;
use transit_model::objects::StopPoint;
use transit_model::Model;

#[derive(Clone)]
pub enum MinimizeArrivalTimeError<Data: DataTrait> {
    NoBoardTime(Data::Trip, Data::Position),
    NoDebarkTime(Data::Trip, Data::Position),
    NoTrip(SecondsSinceDatasetUTCStart, Data::Mission, Data::Position),
}

impl<Data: DataTrait> Debug for MinimizeArrivalTimeError<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MinimizeArrivalTimeError::NoTrip(_, _, _) => {
                write!(f, "NoTrip")
            }
            MinimizeArrivalTimeError::NoBoardTime(_, _) => {
                write!(f, "NoBoardTime")
            }
            MinimizeArrivalTimeError::NoDebarkTime(_, _) => {
                write!(f, "NoDebarkTime")
            }
        }
    }
}

#[derive(Clone)]
pub enum MaximizeDepartureTimeError<Data: DataTrait> {
    NoBoardTime(Data::Trip, Data::Position),
    NoDebarkTime(Data::Trip, Data::Position),
    NoTrip(SecondsSinceDatasetUTCStart, Data::Mission, Data::Position),
}

impl<Data: DataTrait> Debug for MaximizeDepartureTimeError<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaximizeDepartureTimeError::NoTrip(_, _, _) => {
                write!(f, "NoTrip")
            }
            MaximizeDepartureTimeError::NoBoardTime(_, _) => {
                write!(f, "NoBoardTime")
            }
            MaximizeDepartureTimeError::NoDebarkTime(_, _) => {
                write!(f, "NoDebarkTime")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Criteria {
    pub(super) time: SecondsSinceDatasetUTCStart,
    pub(super) nb_of_legs: u8,
    pub(super) fallback_duration: PositiveDuration,
    pub(super) transfers_duration: PositiveDuration,
    pub(super) loads_count: LoadsCount,
}

pub struct Types<Data> {
    _phantom: PhantomData<Data>,
}

impl<'data, Data: DataTrait> TransitTypes for Types<Data> {
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Position = Data::Position;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
}

impl<'data, Data: DataTrait> RequestTypes for Types<Data> {
    type Departure = Departure;

    type Arrival = Arrival;

    type Criteria = Criteria;
}

pub(super) fn parse_datetime(
    datetime: &NaiveDateTime,
    calendar: &Calendar,
) -> Result<SecondsSinceDatasetUTCStart, BadRequest> {
    calendar.from_naive_datetime(datetime).ok_or_else(|| {
        warn!(
            "The requested datetime {:?} is out of bound of the allowed dates. \
                Allowed dates are between {:?} and {:?}.",
            datetime,
            calendar.first_datetime(),
            calendar.last_datetime(),
        );
        BadRequest::RequestedDatetime
    })
}

pub(super) fn parse_departures<Data>(
    departures_stop_point_and_fallback_duration: &[(String, PositiveDuration)],
    model: &Model,
    transit_data: &Data,
) -> Result<Vec<(Data::Stop, PositiveDuration)>, BadRequest>
where
    Data: DataTrait,
{
    parse_departures_filtered(
        departures_stop_point_and_fallback_duration,
        model,
        transit_data,
        |_| true,
    )
}

pub(super) fn parse_departures_filtered<Data, Filter>(
    departures_stop_point_and_fallback_duration: &[(String, PositiveDuration)],
    model: &Model,
    transit_data: &Data,
    filter: Filter,
) -> Result<Vec<(Data::Stop, PositiveDuration)>, BadRequest>
where
    Data: DataTrait,
    Filter: Fn(Idx<StopPoint>) -> bool,
{
    let result: Vec<_> = departures_stop_point_and_fallback_duration
        .iter()
        .enumerate()
        .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
            let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                warn!(
                    "The {}th departure stop point {} is not found in model. \
                            I ignore it.",
                    idx, stop_point_uri
                );
                None
            })?;
            if filter(stop_idx) {
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                    "The {}th departure stop point {} with idx {:?} is not found in transit_data. \
                        I ignore it",
                    idx, stop_point_uri, stop_idx
                );
                    None
                })?;
                Some((stop, *fallback_duration))
            } else {
                None
            }
        })
        .collect();
    if result.is_empty() {
        return Err(BadRequest::NoValidDepartureStop);
    }
    Ok(result)
}

pub(super) fn parse_arrivals<Data>(
    arrivals_stop_point_and_fallback_duration: &[(String, PositiveDuration)],
    model: &Model,
    transit_data: &Data,
) -> Result<Vec<(Data::Stop, PositiveDuration)>, BadRequest>
where
    Data: DataTrait,
{
    parse_arrivals_filtered(
        arrivals_stop_point_and_fallback_duration,
        model,
        transit_data,
        |_| true,
    )
}

pub(super) fn parse_arrivals_filtered<Data, Filter>(
    arrivals_stop_point_and_fallback_duration: &[(String, PositiveDuration)],
    model: &Model,
    transit_data: &Data,
    filter: Filter,
) -> Result<Vec<(Data::Stop, PositiveDuration)>, BadRequest>
where
    Data: DataTrait,
    Filter: Fn(Idx<StopPoint>) -> bool,
{
    let result: Vec<_> = arrivals_stop_point_and_fallback_duration
        .iter()
        .enumerate()
        .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
            let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                warn!(
                    "The {}th arrival stop point {} is not found in model. \
                            I ignore it.",
                    idx, stop_point_uri
                );
                None
            })?;
            if filter(stop_idx) {
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                    "The {}th arrival stop point {} with idx {:?} is not found in transit_data. \
                        I ignore it",
                    idx, stop_point_uri, stop_idx
                );
                    None
                })?;
                Some((stop, *fallback_duration))
            } else {
                None
            }
        })
        .collect();
    if result.is_empty() {
        return Err(BadRequest::NoValidArrivalStop);
    }
    Ok(result)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Departure {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Arrival {
    pub(super) idx: usize,
}

pub struct Departures {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Departures {
    type Item = Departure;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Departure { idx })
    }
}

pub struct Arrivals {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Arrivals {
    type Item = Arrival;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Arrival { idx })
    }
}

pub(super) fn stop_name<Data: DataTrait>(
    stop: &Data::Stop,
    model: &Model,
    transit_data: &Data,
) -> String {
    let stop_point_idx = transit_data.stop_point_idx(stop);
    let stop_point = &model.stop_points[stop_point_idx];
    stop_point.id.clone()
}

pub(super) fn trip_name<Data: DataTrait>(
    trip: &Data::Trip,
    model: &Model,
    transit_data: &Data,
) -> String {
    let vehicle_journey_idx = transit_data.vehicle_journey_idx(trip);
    let date = transit_data.day_of(trip);
    let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
    format!(
        "{}_{}_{}",
        vehicle_journey.id,
        date.to_string(),
        vehicle_journey.route_id
    )
}

pub(super) fn mission_name<Data: DataTrait>(
    mission: &Data::Mission,
    _model: &Model,
    transit_data: &Data,
) -> String {
    let mission_id = transit_data.mission_id(mission);
    format!("{}", mission_id)
}

pub(super) fn position_name<Data: DataTrait>(
    position: &Data::Position,
    mission: &Data::Mission,
    model: &Model,
    transit_data: &Data,
) -> String {
    let stop = transit_data.stop_of(position, mission);
    let stop_name = stop_name(&stop, model, transit_data);
    let mission_name = mission_name(mission, model, transit_data);
    format!("{}_{}", stop_name, mission_name,)
}
