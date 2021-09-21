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

use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use tracing::{debug, trace};
use transit_model::objects::VehicleJourney;
use transit_model::Model;
use typed_index_collection::Idx;

type StopSequence = u32;
type Occupancy = u8;
type VehicleJourneyIdx = Idx<VehicleJourney>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Load {
    Low,
    Medium,
    High,
}

impl Display for Load {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Load::Low => write!(f, "Low"),
            Load::Medium => write!(f, "Medium"),
            Load::High => write!(f, "High"),
        }
    }
}

impl Default for Load {
    fn default() -> Self {
        Load::Medium
    }
}

use std::cmp::Ordering;

fn load_to_int(load: &Load) -> u8 {
    match load {
        Load::Low => 0,
        Load::Medium => 1,
        Load::High => 2,
    }
}

impl Ord for Load {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&load_to_int(self), &load_to_int(other))
    }
}

impl PartialOrd for Load {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LoadsCount {
    pub high: u16,
    pub medium: u16,
    pub low: u16,
}

impl LoadsCount {
    pub fn zero() -> Self {
        Self {
            high: 0,
            medium: 0,
            low: 0,
        }
    }

    pub fn add(&self, load: Load) -> Self {
        let mut high = self.high;
        let mut medium = self.medium;
        let mut low = self.low;
        match load {
            Load::High => {
                high += 1;
            }
            Load::Medium => {
                medium += 1;
            }
            Load::Low => {
                low += 1;
            }
        }
        Self { high, medium, low }
    }

    pub fn total(&self) -> u16 {
        self.high + self.medium + self.low
    }

    pub fn max(&self) -> Load {
        if self.high > 0 {
            return Load::High;
        }
        if self.medium > 0 {
            return Load::Medium;
        }
        Load::Low
    }

    pub fn is_lower(&self, other: &Self) -> bool {
        use Ordering::{Equal, Greater, Less};
        match self.high.cmp(&other.high) {
            Less => true,
            Greater => false,
            Equal => match self.medium.cmp(&other.medium) {
                Less => true,
                Greater => false,
                Equal => match self.low.cmp(&other.low) {
                    Less | Equal => true,
                    Greater => false,
                },
            },
        }
    }
}

impl Default for LoadsCount {
    fn default() -> Self {
        Self::zero()
    }
}

impl Display for LoadsCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "High {}; Medium {}; Low {}; total {}",
            self.high,
            self.medium,
            self.low,
            self.total()
        )
    }
}

fn occupancy_to_load(occupancy: Occupancy) -> Load {
    debug_assert!(occupancy <= 100);
    if occupancy <= 30 {
        Load::Low
    } else if occupancy <= 70 {
        Load::Medium
    } else {
        Load::High
    }
}

pub struct LoadsData {
    per_vehicle_journey: BTreeMap<VehicleJourneyIdx, VehicleJourneyLoads>,
}

struct VehicleJourneyLoads {
    stop_sequence_to_idx: BTreeMap<StopSequence, usize>,
    per_date: BTreeMap<NaiveDate, TripLoads>,
}

struct TripLoads {
    per_stop: Vec<Load>,
}

impl VehicleJourneyLoads {
    fn new<StopSequenceIter>(stop_sequence_iter: StopSequenceIter) -> Self
    where
        StopSequenceIter: Iterator<Item = StopSequence>,
    {
        let mut stop_sequence_to_idx = BTreeMap::new();
        for (idx, stop_sequence) in stop_sequence_iter.enumerate() {
            stop_sequence_to_idx.insert(stop_sequence, idx);
        }
        Self {
            stop_sequence_to_idx,
            per_date: BTreeMap::new(),
        }
    }
}

impl TripLoads {
    fn new(nb_of_stop: usize) -> Self {
        Self {
            per_stop: vec![Load::Medium; nb_of_stop],
        }
    }
}

impl LoadsData {
    pub fn loads(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&[Load]> {
        let vehicle_journey_load = self.per_vehicle_journey.get(vehicle_journey_idx)?;
        let trip_load = vehicle_journey_load.per_date.get(date)?;
        let nb_of_stops = trip_load.per_stop.len();
        Some(&trip_load.per_stop[..(nb_of_stops - 1)])
    }

    pub fn empty() -> Self {
        LoadsData {
            per_vehicle_journey: BTreeMap::new(),
        }
    }

    pub fn new<P: AsRef<Path>>(
        csv_occupancys_filepath: P,
        model: &Model,
    ) -> Result<Self, Box<dyn Error>> {
        let mut loads_data = LoadsData {
            per_vehicle_journey: BTreeMap::new(),
        };
        let filepath = csv_occupancys_filepath.as_ref();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_path(filepath)?;

        let mut record = csv::StringRecord::new();

        while reader.read_record(&mut record)? {
            let is_valid_record = parse_record(&record, model);
            let (vehicle_journey_idx, stop_sequence, occupancy, date) = match is_valid_record {
                Ok((vehicle_journey_idx, stop_sequence, occupancy, date)) => {
                    (vehicle_journey_idx, stop_sequence, occupancy, date)
                }
                Err(_parse_error) => {
                    // trace!(
                    //     "Error reading {:?} at line {} : {} \n. I'll skip this line. ",
                    //     filepath,
                    //     reader.position().line(),
                    //     parse_error
                    // );
                    continue;
                }
            };
            let load = occupancy_to_load(occupancy);

            let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
            let stop_sequence_iter = vehicle_journey
                .stop_times
                .iter()
                .map(|stop_time| stop_time.sequence);
            let nb_of_stop = vehicle_journey.stop_times.len();

            let vehicle_journey_loads = loads_data
                .per_vehicle_journey
                .entry(vehicle_journey_idx)
                .or_insert_with(|| VehicleJourneyLoads::new(stop_sequence_iter));
            let idx = {
                let has_idx = vehicle_journey_loads
                    .stop_sequence_to_idx
                    .get(&stop_sequence);
                if has_idx.is_none() {
                    trace!(
                        "Error reading {:?} at line {}. \n
                        The provided stop_sequence {} is not valid for the vehicle_journey {}.
                        I'll skip this line.",
                        filepath,
                        reader.position().line(),
                        stop_sequence,
                        &record[0]
                    );
                    continue;
                }
                has_idx.unwrap()
            };

            let trip_load = vehicle_journey_loads
                .per_date
                .entry(date)
                .or_insert_with(|| TripLoads::new(nb_of_stop));
            trip_load.per_stop[*idx] = load;
        }

        // loads_data._check(model);

        Ok(loads_data)
    }

    fn _check(&self, model: &Model) {
        // for each vehicle_journey, check that :
        //  - for each valid date, we have occupancy data for every stop_time
        for (vehicle_journey_idx, vehicle_journey) in model.vehicle_journeys.iter() {
            let has_calendar = model.calendars.get(&vehicle_journey.service_id);
            if has_calendar.is_none() {
                continue;
            }
            let calendar = has_calendar.unwrap();

            let has_vehicle_journey_load = self.per_vehicle_journey.get(&vehicle_journey_idx);
            if has_vehicle_journey_load.is_none() {
                debug!(
                    "No occupancy data provided for vehicle_journey {}",
                    vehicle_journey.id
                );
                continue;
            }
            let vehicle_journey_load = has_vehicle_journey_load.unwrap();
            for date in calendar.dates.iter() {
                let has_trip_load = vehicle_journey_load.per_date.get(date);
                if has_trip_load.is_none() {
                    trace!(
                        "No occupancy data provided for vehicle_journey {} on date {}",
                        vehicle_journey.id,
                        date
                    );
                    continue;
                }
            }
        }
    }
}

fn parse_record(
    record: &csv::StringRecord,
    model: &Model,
) -> Result<(VehicleJourneyIdx, StopSequence, Occupancy, NaiveDate), Box<dyn Error>> {
    if record.len() != 4 {
        let msg = format!("Expected 4 fields, but got {}", record.len());
        return Err(From::from(msg));
    }

    let vehicle_journey_idx = {
        let trip_id = &record[0];
        model
            .vehicle_journeys
            .get_idx(trip_id)
            .ok_or_else(|| format!("Cannot find a trip named {} in the ntfs data.", trip_id,))?
    };

    let stop_sequence = {
        let string = &record[1];
        string.parse::<StopSequence>().map_err(|parse_error| {
            format!(
                "Cannot parse the second field (stop_sequence) {} as usize.
                    Parse error {:?}.",
                string, parse_error
            )
        })?
    };

    let occupancy = {
        let occupancy_string = &record[2];
        let occupancy_float = occupancy_string.parse::<f64>().map_err(|parse_error| {
            format!(
                "Cannot parse the third field (occupancy) {} as a float.
                        Parse error {:?}.",
                occupancy_string, parse_error
            )
        })?;
        if occupancy_float.is_infinite()
            || occupancy_float.is_nan()
            || occupancy_float < 0.0f64
            || occupancy_float > 1.0f64
        {
            let msg = format!(
                "The third field {} is not a valid value for occupancy.
                                It should be a float between 0.0 and 1.0.",
                occupancy_string,
            );
            return Err(From::from(msg));
        }
        // the cast is safe because we check above that occupancy_float is between 0.0 and 1.0
        // thus (occupancy_float * 100).trunc is between 0 and 100
        // and thus will fit into an u8
        (occupancy_float * 100.0).trunc() as u8
    };

    let date = {
        let date_string = &record[3];
        NaiveDate::parse_from_str(date_string, "%Y-%m-%d").map_err(|_| {
            format!(
                "The fourth field {} is not a valid date.
                    It should be formatted like 2020-04-17.",
                date_string
            )
        })?
    };

    Ok((vehicle_journey_idx, stop_sequence, occupancy, date))
}
