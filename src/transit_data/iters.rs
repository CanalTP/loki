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

use super::{Stop, Transfer, TransferDurations, TransitData};

use crate::timetables::Timetables as TimetablesTrait;

pub type OutgoingTransfersAtStop<'data> =
    std::slice::Iter<'data, (Stop, TransferDurations, Transfer)>;
pub type IncomingTransfersAtStop<'data> =
    std::slice::Iter<'data, (Stop, TransferDurations, Transfer)>;

impl<Timetables: TimetablesTrait> TransitData<Timetables> {
    pub fn missions_of(&self, stop: &Stop) -> MissionsOfStop<Timetables> {
        let stop_data = self.stop_data(stop);
        MissionsOfStop {
            inner: stop_data.position_in_timetables.iter(),
        }
    }

    pub fn missions_of_filtered<Filter>(
        &self,
        stop: &Stop,
        filter: Filter,
    ) -> MissionsOfStop<Timetables>
    where
        Filter: Fn(&Stop) -> bool,
    {
        if let true = filter(stop) {
            let stop_data = self.stop_data(stop);
            MissionsOfStop {
                inner: stop_data.position_in_timetables.iter(),
            }
        } else {
            MissionsOfStop { inner: [].iter() }
        }
    }

    pub fn outgoing_transfers_at(&self, stop: &Stop) -> OutgoingTransfersAtStop {
        let stop_data = self.stop_data(stop);
        stop_data.outgoing_transfers.iter()
    }

    pub fn incoming_transfers_at(&self, stop: &Stop) -> IncomingTransfersAtStop {
        let stop_data = self.stop_data(stop);
        stop_data.incoming_transfers.iter()
    }
}

pub struct MissionsOfStop<'a, Timetables: TimetablesTrait> {
    inner: std::slice::Iter<'a, (Timetables::Mission, Timetables::Position)>,
}

impl<'a, Timetables: TimetablesTrait> Iterator for MissionsOfStop<'a, Timetables> {
    type Item = (Timetables::Mission, Timetables::Position);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().cloned()
    }
}

impl<'a, Timetables: TimetablesTrait> ExactSizeIterator for MissionsOfStop<'a, Timetables> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
