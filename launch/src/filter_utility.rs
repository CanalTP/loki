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

use crate::loki::Idx;
use loki::transit_model::objects::{CommercialMode, PhysicalMode};
use loki::transit_model::{
    model::GetCorresponding,
    objects::{Line, Network, Route, StopArea, StopPoint, VehicleJourney},
    Model,
};
use relational_types::IdxSet;
use std::collections::HashSet;

#[derive(Debug)]
pub enum FilterType<'a> {
    StopPoint(&'a str),
    StopArea(&'a str),
    Line(&'a str),
    Route(&'a str),
    Network(&'a str),
    PhysicalMode(&'a str),
    CommercialMode(&'a str),
}

pub struct Filters {
    pub forbidden_sp_idx: HashSet<Idx<StopPoint>>,
    pub allowed_sp_idx: HashSet<Idx<StopPoint>>,
    pub forbidden_vj_idx: HashSet<Idx<VehicleJourney>>,
    pub allowed_vj_idx: HashSet<Idx<VehicleJourney>>,
}

pub fn parse_filter(input: &str) -> Option<FilterType> {
    if let Some(stop_point) = input.strip_prefix("stop_point:") {
        return Some(FilterType::StopPoint(stop_point));
    }
    if let Some(stop_area) = input.strip_prefix("stop_area:") {
        return Some(FilterType::StopArea(stop_area));
    }
    if let Some(line) = input.strip_prefix("line:") {
        return Some(FilterType::Line(line));
    }
    if let Some(route) = input.strip_prefix("route:") {
        return Some(FilterType::Route(route));
    }
    if let Some(network) = input.strip_prefix("network:") {
        return Some(FilterType::Network(network));
    }
    if let Some(physical_mode) = input.strip_prefix("physical_mode:") {
        return Some(FilterType::PhysicalMode(physical_mode));
    }
    if let Some(commercial_mode) = input.strip_prefix("commercial_mode:") {
        return Some(FilterType::CommercialMode(commercial_mode));
    }
    None
}

pub fn create_filter_idx(
    model: &Model,
    forbidden_uri: &[String],
    allowed_uri: &[String],
) -> Filters {
    let (forbidden_sp_idx, forbidden_vj_idx) = parse_uri(model, forbidden_uri);
    let (allowed_sp_idx, allowed_vj_idx) = parse_uri(model, allowed_uri);

    Filters {
        forbidden_sp_idx,
        allowed_sp_idx,
        forbidden_vj_idx,
        allowed_vj_idx,
    }
}

fn pt_object_to_vj<T>(
    model: &Model,
    pt_index_set: &IdxSet<T>,
    vj_index_set: &mut HashSet<Idx<VehicleJourney>>,
) where
    IdxSet<T>: GetCorresponding<VehicleJourney>,
{
    let vj_set: IdxSet<VehicleJourney> = pt_index_set.get_corresponding(model);
    vj_index_set.extend(vj_set);
}

fn pt_object_to_sp<T>(
    model: &Model,
    pt_index_set: &IdxSet<T>,
    sp_index_set: &mut HashSet<Idx<StopPoint>>,
) where
    IdxSet<T>: GetCorresponding<StopPoint>,
{
    let sp_set: IdxSet<StopPoint> = pt_index_set.get_corresponding(model);
    sp_index_set.extend(sp_set);
}

fn parse_uri(
    model: &Model,
    uris: &[String],
) -> (HashSet<Idx<StopPoint>>, HashSet<Idx<VehicleJourney>>) {
    let mut set_sp_idx = HashSet::new();
    let mut set_vj_idx = HashSet::new();

    let mut set_sa_idx: IdxSet<StopArea> = IdxSet::new();
    let mut set_line_idx: IdxSet<Line> = IdxSet::new();
    let mut set_route_idx: IdxSet<Route> = IdxSet::new();
    let mut set_network_idx: IdxSet<Network> = IdxSet::new();
    let mut set_physical_mode_idx: IdxSet<PhysicalMode> = IdxSet::new();
    let mut set_commercial_mode_idx: IdxSet<CommercialMode> = IdxSet::new();

    for str in uris {
        let parsed_str = parse_filter(str.as_str());
        match parsed_str {
            Some(FilterType::StopPoint(sp)) => {
                let sp_idx = model.stop_points.get_idx(sp);
                if let Some(idx) = sp_idx {
                    set_sp_idx.insert(idx);
                }
            }
            Some(FilterType::StopArea(sa_uri)) => {
                let opt_idx = model.stop_areas.get_idx(sa_uri);
                if let Some(idx) = opt_idx {
                    set_sa_idx.insert(idx);
                }
            }
            Some(FilterType::Line(line)) => {
                let line_idx = model.lines.get_idx(line);
                if let Some(idx) = line_idx {
                    set_line_idx.insert(idx);
                }
            }
            Some(FilterType::Route(route)) => {
                let route_idx = model.routes.get_idx(route);
                if let Some(idx) = route_idx {
                    set_route_idx.insert(idx);
                }
            }
            Some(FilterType::Network(network)) => {
                let network_idx = model.networks.get_idx(network);
                if let Some(idx) = network_idx {
                    set_network_idx.insert(idx);
                }
            }
            Some(FilterType::PhysicalMode(physical_mode)) => {
                let physical_mode_idx = model.physical_modes.get_idx(physical_mode);
                if let Some(idx) = physical_mode_idx {
                    set_physical_mode_idx.insert(idx);
                }
            }
            Some(FilterType::CommercialMode(commercial_mode)) => {
                let commercial_mode_idx = model.commercial_modes.get_idx(commercial_mode);
                if let Some(idx) = commercial_mode_idx {
                    set_commercial_mode_idx.insert(idx);
                }
            }
            _ => (),
        }
    }

    pt_object_to_sp(model, &set_sa_idx, &mut set_sp_idx);
    pt_object_to_vj(model, &set_line_idx, &mut set_vj_idx);
    pt_object_to_vj(model, &set_route_idx, &mut set_vj_idx);
    pt_object_to_vj(model, &set_network_idx, &mut set_vj_idx);
    pt_object_to_vj(model, &set_physical_mode_idx, &mut set_vj_idx);
    pt_object_to_vj(model, &set_commercial_mode_idx, &mut set_vj_idx);

    (set_sp_idx, set_vj_idx)
}
