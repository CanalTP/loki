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

mod utils;
use failure::Error;
use launch::config::{ComparatorType, DataImplem};
use launch::loki::transit_model::Model;
use rstest::fixture;
use rstest::rstest;
use utils::model_builder::ModelBuilder;
use utils::{build_and_solve, make_pt_from_vehicle, Config};

#[fixture]
pub fn fixture_model() -> Model {
    ModelBuilder::new("2020-01-01", "2020-01-02")
        .route("R1", |r| r.name = "R1".into())
        .route("R2", |r| r.name = "R2".into())
        .route("R3", |r| r.name = "R3".into())
        .route("R4", |r| r.name = "R3".into())
        .vj("toto", |vj_builder| {
            vj_builder
                .route("R1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("tata", |vj_builder| {
            vj_builder
                .route("R2")
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00");
        })
        .vj("titi", |vj_builder| {
            vj_builder
                .route("R3")
                .st("H", "10:35:00")
                .st("I", "10:40:00")
                .st("J", "10:45:00");
        })
        .vj("txtx", |vj_builder| {
            vj_builder
                .route("R4")
                .st("C", "10:35:00")
                .st("J", "10:55:00");
        })
        .add_transfer("B", "F", "00:02:00")
        .add_transfer("G", "H", "00:02:00")
        .add_transfer("C", "I", "00:02:00")
        .add_transfer("C", "C", "00:02:00")
        .build()
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_no_filter(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: Model,
) -> Result<(), Error> {
    utils::init_logger();

    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        ..config
    };

    let responses = build_and_solve(&fixture_model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&fixture_model), "toto");
    let (from_sp, to_sp) = make_pt_from_vehicle(vehicle_sec, &fixture_model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "C");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_forbidden_stop_point(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: Model,
) -> Result<(), Error> {
    utils::init_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        forbidden_uri: vec!["stop_point:C"],
        ..config
    };

    let responses = build_and_solve(&fixture_model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 7);
    assert_eq!(journey.connections.len(), 2);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&fixture_model), "toto");
    let (from_sp, to_sp) = make_pt_from_vehicle(vehicle_sec, &fixture_model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "B");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_allowed_stop_point(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: Model,
) -> Result<(), Error> {
    utils::init_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        allowed_uri: vec![
            "stop_point:A",
            "stop_point:B",
            "stop_point:F",
            "stop_point:G",
            "stop_point:H",
            "stop_point:J",
        ],
        ..config
    };

    let responses = build_and_solve(&fixture_model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 7);
    assert_eq!(journey.connections.len(), 2);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&fixture_model), "toto");
    let (from_sp, to_sp) = make_pt_from_vehicle(vehicle_sec, &fixture_model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "B");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_forbidden_route(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: Model,
) -> Result<(), Error> {
    utils::init_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        forbidden_uri: vec!["route:R2", "route:R3"],
        ..config
    };

    let responses = build_and_solve(&fixture_model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&fixture_model), "toto");
    let (from_sp, to_sp) = make_pt_from_vehicle(vehicle_sec, &fixture_model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "C");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_allowed_route(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: Model,
) -> Result<(), Error> {
    utils::init_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        allowed_uri: vec!["route:R1", "route:R4"],
        ..config
    };

    let responses = build_and_solve(&fixture_model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&fixture_model), "toto");
    let (from_sp, to_sp) = make_pt_from_vehicle(vehicle_sec, &fixture_model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "C");

    Ok(())
}
