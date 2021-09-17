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
#![allow(dead_code)]
pub mod model_builder;

use env_logger::Env;
use failure::{format_err, Error};
use launch::config;
use launch::config::launch_params::default_transfer_duration;
use launch::datetime::DateTimeRepresent;
use launch::filter_utility::create_filter_idx;
use launch::loki::response::VehicleSection;
use launch::loki::timetables::{Timetables as TimetablesTrait, TimetablesIter};
use launch::loki::transit_data_filtered::DataFilter;
use launch::loki::{response, Idx, RequestInput, StopPoint};
use launch::solver::Solver;
use loki::chrono::TimeZone;
use loki::log::debug;
use loki::transit_model::Model;
use loki::{chrono_tz, TransitData};
use loki::{DailyData, NaiveDateTime, PeriodicData, PeriodicSplitVjData};
use loki::{LoadsData, PositiveDuration};
use model_builder::AsDateTime;
use std::fmt::Debug;

pub fn init_logger() {
    let _ = env_logger::Builder::from_env(
        // use log level specified by RUST_LOG env var if set
        //  and default to the "debug" level when RUST_LOG is not set
        Env::default().default_filter_or("debug"),
    )
    .is_test(true)
    .try_init();
}

pub struct Config {
    pub request_params: config::RequestParams,

    pub datetime: NaiveDateTime,

    pub datetime_represent: DateTimeRepresent,

    pub comparator_type: config::ComparatorType,

    pub data_implem: config::DataImplem,

    pub default_transfer_duration: PositiveDuration,

    /// name of the start stop_area
    pub start: String,

    /// name of the end stop_area
    pub end: String,

    // Allowed_uri
    pub allowed_uri: Vec<String>,

    // Forbidden_uri
    pub forbidden_uri: Vec<String>,
}

impl Config {
    pub fn new(datetime: impl AsDateTime, start: &str, end: &str) -> Self {
        Self::new_timezoned(datetime, &model_builder::DEFAULT_TIMEZONE, start, end)
    }

    pub fn new_timezoned(
        datetime: impl AsDateTime,
        timezone: &chrono_tz::Tz,
        start: &str,
        end: &str,
    ) -> Self {
        let naive_datetime = datetime.as_datetime();
        let timezoned_datetime = timezone.from_local_datetime(&naive_datetime).unwrap();
        let utc_datetime = timezoned_datetime.naive_utc();
        Config {
            request_params: Default::default(),
            datetime: utc_datetime,
            datetime_represent: Default::default(),
            comparator_type: Default::default(),
            data_implem: Default::default(),
            default_transfer_duration: default_transfer_duration(),
            start: start.into(),
            end: end.into(),
            allowed_uri: Default::default(),
            forbidden_uri: Default::default(),
        }
    }
}

fn make_request_from_config(model: &Model, config: &Config) -> Result<RequestInput, Error> {
    let datetime = config.datetime;

    let start_stop_point_uri = &config.start;
    let end_stop_point_uri = &config.end;

    let filters = create_filter_idx(model, &config.forbidden_uri, &config.forbidden_uri);

    let request_input = RequestInput {
        datetime,
        departures_stop_point_and_fallback_duration: vec![(
            start_stop_point_uri.clone(),
            PositiveDuration::zero(),
        )],
        arrivals_stop_point_and_fallback_duration: vec![(
            end_stop_point_uri.clone(),
            PositiveDuration::zero(),
        )],
        leg_arrival_penalty: config.request_params.leg_arrival_penalty,
        leg_walking_penalty: config.request_params.leg_walking_penalty,
        max_nb_of_legs: config.request_params.max_nb_of_legs,
        max_journey_duration: config.request_params.max_journey_duration,
        too_late_threshold: config.request_params.too_late_threshold,
        filters: DataFilter {
            forbidden_sp_idx: filters.forbidden_sp_idx.into_iter().collect(),
            allowed_sp_idx: filters.allowed_sp_idx.into_iter().collect(),
            forbidden_vj_idx: filters.forbidden_vj_idx.into_iter().collect(),
            allowed_vj_idx: filters.allowed_vj_idx.into_iter().collect(),
        },
    };
    Ok(request_input)
}

pub fn build_and_solve(
    model: &Model,
    loads_data: &LoadsData,
    config: &Config,
) -> Result<Vec<response::Response>, Error> {
    match config.data_implem {
        config::DataImplem::Periodic => {
            build_and_solve_inner::<PeriodicData>(model, loads_data, config)
        }
        config::DataImplem::Daily => build_and_solve_inner::<DailyData>(model, loads_data, config),
        config::DataImplem::PeriodicSplitVj => {
            build_and_solve_inner::<PeriodicSplitVjData>(model, loads_data, config)
        }
    }
}

fn build_and_solve_inner<Timetables>(
    model: &Model,
    loads_data: &LoadsData,
    config: &Config,
) -> Result<Vec<response::Response>, Error>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    use loki::DataTrait;
    let data: TransitData<Timetables> =
        launch::read::build_transit_data(model, loads_data, &config.default_transfer_duration);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let request_input = make_request_from_config(model, config)?;

    let responses = solver.solve_request(
        &data,
        model,
        &request_input,
        &config.comparator_type,
        &config.datetime_represent,
    )?;
    for response in responses.iter() {
        debug!("{}", response.print(model)?);
    }
    Ok(responses)
}

pub fn make_pt_from_vehicle<'a>(
    vehicle_section: &VehicleSection,
    model: &'a Model,
) -> Result<(&'a StopPoint, &'a StopPoint), Error> {
    let vehicle_journey = &model.vehicle_journeys[vehicle_section.vehicle_journey];

    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let from_stoptime = vehicle_journey
        .stop_times
        .get(from_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let from_stop_point_idx = from_stoptime.stop_point_idx;
    let from_stop_point = make_stop_point(&from_stop_point_idx, model);

    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stoptime = vehicle_journey
        .stop_times
        .get(to_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let to_stop_point_idx = to_stoptime.stop_point_idx;
    let to_stop_point = make_stop_point(&to_stop_point_idx, model);

    Ok((from_stop_point, to_stop_point))
}

pub fn make_stop_point<'a>(stop_point_idx: &Idx<StopPoint>, model: &'a Model) -> &'a StopPoint {
    &model.stop_points[*stop_point_idx]
}
