// Copyright (C) 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

//! Provides an easy way to create a `crate::Model`
//!
//! ```
//! # use loki::modelbuilder::ModelBuilder;
//!
//! # fn main() {
//!  let model = ModelBuilder::default()
//!      .vj("toto", |vj| {
//!          vj.route("1")
//!            .st("A", "10:00:00", "10:01:00")
//!            .st("B", "11:00:00", "11:01:00");
//!      })
//!      .vj("tata", |vj| {
//!          vj.st("A", "10:00:00", "10:01:00")
//!            .st("D", "11:00:00", "11:01:00");
//!      })
//!      .build();
//! # }
//! ```

use loki::chrono_tz::{self};
use loki::transit_model::model::Collections;
use loki::transit_model::objects::{
    Calendar, CommercialMode, Date, Line, Network, PhysicalMode, Route, StopPoint, StopTime, Time,
    Transfer, ValidityPeriod, VehicleJourney,
};
use loki::transit_model::Model;
use loki::typed_index_collection::Idx;
use loki::NaiveDateTime;

const DEFAULT_CALENDAR_ID: &str = "default_service";
const DEFAULT_ROUTE_ID: &str = "default_route";
const DEFAULT_LINE_ID: &str = "default_line";
const DEFAULT_NETWORK_ID: &str = "default_network";
const DEFAULT_COMMERCIAL_MODE_ID: &str = "default_commercial_mode";
const DEFAULT_PHYSICAL_MODE_ID: &str = "default_physical_mode";

pub const DEFAULT_TIMEZONE: chrono_tz::Tz = chrono_tz::UTC;

/// Builder used to easily create a `Model`
/// Note: if not explicitly set all the vehicule journeys
/// will be attached to a default calendar starting 2020-01-01
///
pub struct ModelBuilder {
    collections: Collections,
    validity_period: ValidityPeriod,
}

/// Builder used to create and modify a new VehicleJourney
/// Note: if not explicitly set, the vehicule journey
/// will be attached to a default calendar starting 2020-01-01
pub struct VehicleJourneyBuilder<'a> {
    model: &'a mut ModelBuilder,
    vj_idx: Idx<VehicleJourney>,
    info: VehicleJourneyInfo,
}

#[derive(PartialEq, Eq)]
pub enum VehicleJourneyInfo {
    Route(String),
    Line(String),
    Network(String),
    CommercialMode(String),
    PhysicalMode(String),
    Timezone(chrono_tz::Tz),
    None,
}

impl Default for ModelBuilder {
    fn default() -> Self {
        let date = "2020-01-01";
        Self::new(date, date)
    }
}

impl<'a> ModelBuilder {
    pub fn new(start_validity_period: impl AsDate, end_validity_period: impl AsDate) -> Self {
        let start_date = start_validity_period.as_date();
        let end_date = end_validity_period.as_date();
        let model_builder = Self {
            validity_period: ValidityPeriod {
                start_date,
                end_date,
            },
            collections: Collections::default(),
        };

        assert!(start_date <= end_date);
        let dates: Vec<_> = start_date
            .iter_days()
            .take_while(|date| *date <= end_date)
            .collect();

        model_builder.default_calendar(&dates)
    }

    /// Add a new VehicleJourney to the model
    ///
    /// ```
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///        .vj("toto", |vj_builder| {
    ///            vj_builder
    ///                .st("A", "10:00:00")
    ///                .st("B", "11:00:00");
    ///        })
    ///        .vj("tata", |vj_builder| {
    ///            vj_builder
    ///                .st("C", "08:00:00")
    ///                .st("B", "09:00:00");
    ///        })
    ///        .build();
    /// # }
    /// ```
    pub fn vj<F>(mut self, name: &str, mut vj_initer: F) -> Self
    where
        F: FnMut(VehicleJourneyBuilder),
    {
        let new_vj = VehicleJourney {
            id: name.into(),
            service_id: DEFAULT_CALENDAR_ID.to_string(),
            route_id: DEFAULT_ROUTE_ID.to_string(),
            ..Default::default()
        };
        let vj_idx = self
            .collections
            .vehicle_journeys
            .push(new_vj)
            .unwrap_or_else(|_| panic!("vj {} already exists", name));

        let vj = &self.collections.vehicle_journeys[vj_idx];

        {
            let mut dataset = self.collections.datasets.get_or_create(&vj.dataset_id);
            dataset.start_date = self.validity_period.start_date;
            dataset.end_date = self.validity_period.end_date;
        }

        let vj_builder = VehicleJourneyBuilder {
            model: &mut self,
            vj_idx,
            info: VehicleJourneyInfo::None,
        };

        vj_initer(vj_builder);
        self
    }

    /// Add a new Route to the model
    ///
    /// ```
    ///
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///      .route("l1", |r| {
    ///             r.name = "ligne 1".to_owned();
    ///         })
    ///      .vj("toto", |vj| {
    ///          vj.route("l1")
    ///            .st("A", "10:00:00")
    ///            .st("B", "11:00:00");
    ///      })
    ///      .build();
    /// # }
    /// ```
    pub fn route<F>(mut self, id: &str, mut route_initer: F) -> Self
    where
        F: FnMut(&mut Route),
    {
        self.collections.routes.get_or_create_with(id, || {
            let mut r = Route::default();
            route_initer(&mut r);
            r
        });
        self
    }

    pub fn network<F>(mut self, id: &str, mut network_initer: F) -> Self
    where
        F: FnMut(&mut Network),
    {
        self.collections.networks.get_or_create_with(id, || {
            let mut n = Network::default();
            network_initer(&mut n);
            n
        });
        self
    }

    pub fn line<F>(mut self, id: &str, mut line_initer: F) -> Self
    where
        F: FnMut(&mut Line),
    {
        self.collections.lines.get_or_create_with(id, || {
            let mut l = Line::default();
            line_initer(&mut l);
            l
        });
        self
    }

    pub fn commercial_mode<F>(mut self, id: &str, mut initer: F) -> Self
    where
        F: FnMut(&mut CommercialMode),
    {
        self.collections
            .commercial_modes
            .get_or_create_with(id, || {
                let mut c = CommercialMode::default();
                initer(&mut c);
                c
            });
        self
    }

    pub fn physical_mode<F>(mut self, id: &str, mut initer: F) -> Self
    where
        F: FnMut(&mut PhysicalMode),
    {
        self.collections.physical_modes.get_or_create_with(id, || {
            let mut p = PhysicalMode::default();
            initer(&mut p);
            p
        });
        self
    }

    /// Add a new Calendar or change an existing one
    ///
    /// Note: if the date are in strings not in the right format, this conversion will fail
    ///
    /// ```
    /// # use transit_model::objects::Date;
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///      .calendar("c1", &["2020-01-01", "2020-01-02"])
    ///      .calendar("default_service", &[Date::from_ymd(2019, 2, 6)])
    ///      .vj("toto", |vj| {
    ///          vj.calendar("c1")
    ///            .st("A", "10:00:00")
    ///            .st("B", "11:00:00");
    ///      })
    ///      .build();
    /// # }
    /// ```
    pub fn calendar(mut self, id: &str, dates: &[impl AsDate]) -> Self {
        {
            let mut c = self.collections.calendars.get_or_create(id);
            for d in dates {
                c.dates.insert(d.as_date());
            }
        }
        self
    }

    /// Change the default Calendar
    /// If not explicitly set, all vehicule journeys will be linked
    /// to this calendar
    ///
    /// ```
    /// # use transit_model::objects::Date;
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///      .default_calendar(&["2020-01-01"])
    ///      .vj("toto", |vj| {
    ///          vj
    ///            .st("A", "10:00:00")
    ///            .st("B", "11:00:00");
    ///      })
    ///      .build();
    /// # }
    /// ```
    pub fn default_calendar(self, dates: &[impl AsDate]) -> Self {
        self.calendar(DEFAULT_CALENDAR_ID, dates)
    }
    /// Add a new Calendar to the model
    ///
    /// ```
    /// # use transit_model::objects::Date;
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///      .calendar_mut("c1", |c| {
    ///             c.dates.insert(Date::from_ymd(2019, 2, 6));
    ///         })
    ///      .vj("toto", |vj| {
    ///          vj.calendar("c1")
    ///            .st("A", "10:00:00")
    ///            .st("B", "11:00:00");
    ///      })
    ///      .build();
    /// # }
    /// ```
    pub fn calendar_mut<F>(mut self, id: &str, mut calendar_initer: F) -> Self
    where
        F: FnMut(&mut Calendar),
    {
        self.collections.calendars.get_or_create_with(id, || {
            let mut c = Calendar::default();
            calendar_initer(&mut c);
            c
        });
        self
    }

    pub fn add_transfer(
        mut self,
        from_stop_id: &str,
        to_stop_id: &str,
        transfer_duration: impl IntoTime,
    ) -> Self {
        let duration = transfer_duration.into_time().total_seconds();
        self.collections.transfers.push(Transfer {
            from_stop_id: from_stop_id.to_string(),
            to_stop_id: to_stop_id.to_string(),
            min_transfer_time: Some(duration),
            real_min_transfer_time: Some(duration),
            equipment_id: None,
        });
        self
    }

    /// Consume the builder to create a navitia model
    pub fn build(self) -> Model {
        Model::new(self.collections).unwrap()
    }
}

pub trait IntoTime {
    fn into_time(&self) -> Time;
}

impl IntoTime for Time {
    fn into_time(&self) -> Time {
        *self
    }
}

impl IntoTime for &Time {
    fn into_time(&self) -> Time {
        **self
    }
}

impl IntoTime for &str {
    // Note: if the string is not in the right format, this conversion will fail
    fn into_time(&self) -> Time {
        self.parse().expect("invalid time format")
    }
}

pub trait AsDate {
    fn as_date(&self) -> Date;
}

impl AsDate for Date {
    fn as_date(&self) -> Date {
        *self
    }
}

impl AsDate for &Date {
    fn as_date(&self) -> Date {
        **self
    }
}

impl AsDate for &str {
    // Note: if the string is not in the right format, this conversion will fail
    fn as_date(&self) -> Date {
        self.parse().expect("invalid date format")
    }
}

pub trait AsDateTime {
    fn as_datetime(&self) -> NaiveDateTime;
}

impl AsDateTime for &str {
    fn as_datetime(&self) -> NaiveDateTime {
        self.parse().expect("invalid datetime format")
    }
}

impl AsDateTime for NaiveDateTime {
    fn as_datetime(&self) -> NaiveDateTime {
        *self
    }
}

impl AsDateTime for &NaiveDateTime {
    fn as_datetime(&self) -> NaiveDateTime {
        **self
    }
}

impl<'a> VehicleJourneyBuilder<'a> {
    fn find_or_create_sp(&mut self, sp: &str) -> Idx<StopPoint> {
        self.model
            .collections
            .stop_points
            .get_idx(sp)
            .unwrap_or_else(|| {
                let sa_id = format!("sa:{}", sp);
                let new_sp = StopPoint {
                    id: sp.to_owned(),
                    name: sp.to_owned(),
                    stop_area_id: sa_id.clone(),
                    ..Default::default()
                };

                self.model.collections.stop_areas.get_or_create(&sa_id);

                self.model
                    .collections
                    .stop_points
                    .push(new_sp)
                    .unwrap_or_else(|_| panic!("stoppoint {} already exists", sp))
            })
    }

    /// add a StopTime to the vehicle journey
    ///
    /// Note: if the arrival/departure are given in string
    /// not in the right format, this conversion will fail
    ///
    /// ```
    ///
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///        .vj("toto", |vj_builder| {
    ///            vj_builder
    ///                .st("A", "10:00:00")
    ///                .st("B", "11:00:00");
    ///        })
    ///        .build();
    /// # }
    /// ```
    pub fn st(self, name: &str, arrival: impl IntoTime) -> Self {
        self.st_mut(name, arrival.into_time(), arrival.into_time(), |_st| {})
    }

    pub fn st_mut<F>(
        mut self,
        name: &str,
        arrival: impl IntoTime,
        departure: impl IntoTime,
        st_muter: F,
    ) -> Self
    where
        F: FnOnce(&mut StopTime),
    {
        {
            let stop_point_idx = self.find_or_create_sp(name);
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            let sequence = vj.stop_times.len() as u32;
            let mut stop_time = StopTime {
                stop_point_idx,
                sequence,
                arrival_time: arrival.into_time(),
                departure_time: departure.into_time(),
                boarding_duration: 0u16,
                alighting_duration: 0u16,
                pickup_type: 0u8,
                drop_off_type: 0u8,
                datetime_estimated: false,
                local_zone_id: None,
                precision: None,
            };
            st_muter(&mut stop_time);

            vj.stop_times.push(stop_time);
        }

        self
    }

    /// Set the route of the vj
    ///
    /// ```
    ///
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///        .route("1", |r| {
    ///            r.name = "bob".into();
    ///        })
    ///        .vj("toto", |vj_builder| {
    ///            vj_builder.route("1");
    ///        })
    ///        .build();
    /// # }
    /// ```
    pub fn route(mut self, id: &str) -> Self {
        assert!(
            self.info == VehicleJourneyInfo::None,
            "You cannot specify two different info for a vehicle journey"
        );
        self.info = VehicleJourneyInfo::Route(id.to_string());

        self
    }

    pub fn line(mut self, id: &str) -> Self {
        assert!(
            self.info == VehicleJourneyInfo::None,
            "You cannot specify two different info for a vehicle journey"
        );
        self.info = VehicleJourneyInfo::Line(id.to_string());

        self
    }

    pub fn network(mut self, id: &str) -> Self {
        {
            assert!(
                self.info == VehicleJourneyInfo::None,
                "You cannot specify two different info for a vehicle journey"
            );
            self.info = VehicleJourneyInfo::Network(id.to_string());
        }

        self
    }

    pub fn timezone(mut self, timezone: &chrono_tz::Tz) -> Self {
        {
            assert!(
                self.info == VehicleJourneyInfo::None,
                "You cannot specify two different info for a vehicle journey"
            );
            self.info = VehicleJourneyInfo::Timezone(*timezone);
        }

        self
    }

    pub fn commercial_mode(mut self, id: &str) -> Self {
        {
            assert!(
                self.info == VehicleJourneyInfo::None,
                "You cannot specify two different info for a vehicle journey"
            );
            self.info = VehicleJourneyInfo::CommercialMode(id.to_string());
        }

        self
    }

    pub fn physical_mode(mut self, id: &str) -> Self {
        {
            assert!(
                self.info == VehicleJourneyInfo::None,
                "You cannot specify two different info for a vehicle journey"
            );
            self.info = VehicleJourneyInfo::PhysicalMode(id.to_string());
        }

        self
    }

    /// Set the calendar (service_id) of the vj
    ///
    /// ```
    /// # use transit_model::objects::Date;
    /// # use loki::modelbuilder::ModelBuilder;
    ///
    /// # fn main() {
    /// let model = ModelBuilder::default()
    ///        .calendar("c1", &["2021-01-07"])
    ///        .vj("toto", |vj_builder| {
    ///            vj_builder.calendar("c1");
    ///        })
    ///        .build();
    /// # }
    /// ```
    pub fn calendar(self, id: &str) -> Self {
        {
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            vj.service_id = id.to_owned();
        }

        self
    }

    pub fn block_id(self, block_id: &str) -> Self {
        {
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            vj.block_id = Some(block_id.to_owned());
        }
        self
    }
}

impl<'a> Drop for VehicleJourneyBuilder<'a> {
    fn drop(&mut self) {
        use std::ops::DerefMut;
        let collections = &mut self.model.collections;
        // add the missing objects to the model (routes, lines, ...)
        let mut new_vj = collections.vehicle_journeys.index_mut(self.vj_idx);
        let dataset = collections.datasets.get_or_create(&new_vj.dataset_id);
        collections
            .contributors
            .get_or_create(&dataset.contributor_id);

        collections.companies.get_or_create(&new_vj.company_id);
        collections.calendars.get_or_create(&new_vj.service_id);

        collections
            .physical_modes
            .get_or_create(&new_vj.physical_mode_id);

        let route_id = match &self.info {
            VehicleJourneyInfo::Route(id) => id.clone(),
            VehicleJourneyInfo::Line(_)
            | VehicleJourneyInfo::Network(_)
            | VehicleJourneyInfo::Timezone(_) => format!("route_{}", new_vj.id),
            _ => DEFAULT_ROUTE_ID.to_string(),
        };

        new_vj.deref_mut().route_id = route_id;

        let mut route = collections.routes.get_or_create(&new_vj.route_id);
        let line_id = match &self.info {
            VehicleJourneyInfo::Line(id) => id.clone(),
            VehicleJourneyInfo::Network(_) | VehicleJourneyInfo::Timezone(_) => {
                format!("line_{}", new_vj.id)
            }
            _ => DEFAULT_LINE_ID.to_string(),
        };
        route.deref_mut().line_id = line_id.clone();
        let mut line = collections.lines.get_or_create(&line_id);
        collections
            .commercial_modes
            .get_or_create(&line.commercial_mode_id);

        let network_id = match &self.info {
            VehicleJourneyInfo::Network(id) => id.clone(),
            VehicleJourneyInfo::Timezone(_) => format!("network_{}", new_vj.id),
            _ => DEFAULT_NETWORK_ID.to_string(),
        };
        line.deref_mut().network_id = network_id.clone();

        let commercial_mode_id = match &self.info {
            VehicleJourneyInfo::CommercialMode(id) => id.clone(),
            _ => DEFAULT_COMMERCIAL_MODE_ID.to_string(),
        };
        line.deref_mut().commercial_mode_id = commercial_mode_id;

        let physical_mode_id = match &self.info {
            VehicleJourneyInfo::PhysicalMode(id) => id.clone(),
            _ => DEFAULT_PHYSICAL_MODE_ID.to_string(),
        };
        new_vj.deref_mut().physical_mode_id = physical_mode_id;

        let timezone = match &self.info {
            VehicleJourneyInfo::Timezone(timezone) => Some(*timezone),
            _ => Some(DEFAULT_TIMEZONE),
        };
        collections.networks.get_or_create_with(&network_id, || {
            use loki::typed_index_collection::WithId;
            let mut network = Network::with_id(&network_id);
            network.timezone = timezone;
            network
        });
    }
}
