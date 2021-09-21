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

use crate::time::SECONDS_IN_A_DAY;

use super::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    SecondsSinceUTCDayStart, MAX_DAYS_IN_CALENDAR, MAX_SECONDS_IN_TIMEZONED_DAY,
    MAX_SECONDS_IN_UTC_DAY, MAX_TIMEZONE_OFFSET,
};
use chrono::{NaiveDate, NaiveDateTime};
use chrono_tz::Tz as Timezone;
use std::convert::TryFrom;

impl Calendar {
    pub fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        assert!(first_date <= last_date);
        let last_day_offset_i64: i64 = (last_date - first_date).num_days();
        assert!(
            last_day_offset_i64 < MAX_DAYS_IN_CALENDAR as i64,
            "Trying to construct a calendar with {:#} days \
            which is more than the maximum allowed of {:#} days",
            last_day_offset_i64,
            MAX_DAYS_IN_CALENDAR
        );

        // unwrap here is safe because :
        // - last_day_offset >=0 since we asserted above that first_date <= last_date
        // - last_day_offset < MAX_DAYS_IN_CALENDAR < u16::MAX
        let last_day_offset: u16 = TryFrom::try_from(last_day_offset_i64).unwrap();

        Self {
            first_date,
            last_date,
            last_day_offset,
        }
    }

    pub fn nb_of_days(&self) -> u16 {
        // +1 will not overflow since we ensured that
        //  last_day_offset < MAX_DAYS_IN_CALENDAR < u16::MAX
        // in new()
        self.last_day_offset + 1
    }

    pub fn days(&self) -> DaysIter {
        let nb_of_days = self.nb_of_days();
        DaysIter {
            inner: 0..nb_of_days,
        }
    }

    /// The first datetime that can be obtained
    pub fn first_datetime(&self) -> NaiveDateTime {
        self.first_date.and_hms(0, 0, 0)
            - chrono::Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY))
    }

    pub fn last_datetime(&self) -> NaiveDateTime {
        self.last_date.and_hms(0, 0, 0)
            + chrono::Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY))
    }

    pub fn contains_datetime(&self, datetime: &NaiveDateTime) -> bool {
        *datetime >= self.first_datetime() && *datetime <= self.last_datetime()
    }

    pub fn first_date(&self) -> &NaiveDate {
        &self.first_date
    }

    pub fn last_date(&self) -> &NaiveDate {
        &self.last_date
    }

    pub fn contains_date(&self, date: &NaiveDate) -> bool {
        self.first_date <= *date && *date <= self.last_date
    }

    pub fn to_naive_date(&self, day: &DaysSinceDatasetStart) -> NaiveDate {
        *self.first_date() + chrono::Duration::days(day.days as i64)
    }

    pub fn to_string(&self, seconds: &SecondsSinceDatasetUTCStart) -> String {
        let datetime = self.to_naive_datetime(seconds);
        datetime.format("%Y%m%dT%H%M%S").to_string()
    }

    pub fn to_pretty_string(&self, seconds: &SecondsSinceDatasetUTCStart) -> String {
        let datetime = self.to_naive_datetime(seconds);
        datetime.format("%H:%M:%S %d-%b-%y").to_string()
    }

    pub fn to_naive_datetime(&self, seconds: &SecondsSinceDatasetUTCStart) -> NaiveDateTime {
        self.first_datetime() + chrono::Duration::seconds(i64::from(seconds.seconds))
    }

    pub fn date_to_days_since_start(&self, date: &NaiveDate) -> Option<DaysSinceDatasetStart> {
        self.date_to_offset(date)
            .map(|offset| DaysSinceDatasetStart { days: offset })
    }

    pub(super) fn date_to_offset(&self, date: &NaiveDate) -> Option<u16> {
        if *date < self.first_date || *date > self.last_date {
            None
        } else {
            let offset_64: i64 = (*date - self.first_date).num_days();
            // should be safe because :
            //  - we check that offset_64 is positive above when testing if date < self.first_date
            //  - we check that offset_64 is smaller than u16::MAX because at construction of Calendars
            //    we ensure that (last_date - first_date).num_days() < u16::MAX
            //    and we check above that date <= self.last_date
            let offset = offset_64 as u16;
            Some(offset)
        }
    }

    fn decompose<Timezone: chrono::offset::TimeZone>(
        &self,
        datetime_to_decompose: chrono::DateTime<Timezone>,
        reference_date: &chrono::Date<Timezone>,
    ) -> Option<(DaysSinceDatasetStart, SecondsSinceTimezonedDayStart)> {
        let reference_datetime = reference_date.and_hms(12, 0, 0) - chrono::Duration::hours(12);
        let seconds_i64 = (datetime_to_decompose - reference_datetime).num_seconds();

        let has_seconds = SecondsSinceTimezonedDayStart::from_seconds_i64(seconds_i64);
        let has_reference_day = self.date_to_days_since_start(&reference_date.naive_local());
        match (has_reference_day, has_seconds) {
            (Some(reference_day), Some(seconds)) => Some((reference_day, seconds)),
            _ => None,
        }
    }

    // returns an iterator that provides all decompositions of `second_in_dataset_start`
    // of the form (day, time_in_timezoned_day) such that :
    //  - `day` belongs to the calendar
    //  - `time_in_timezoned_day` belongs to the interval `[min_seconds_since_timezoned_day_start, max_seconds_since_timezoned_day_start]`
    pub fn decompositions<'a>(
        &'a self,
        seconds_since_dataset_start: &SecondsSinceDatasetUTCStart,
        timezone: &Timezone,
        max_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
        min_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
    ) -> impl Iterator<Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart)> + 'a {
        // will advance the date until `time_in_timezoned_day` becomes smaller than `min_seconds_since_timezoned_day_start`
        let forward_iter = ForwardDecompose::new(
            seconds_since_dataset_start,
            min_seconds_since_timezoned_day_start,
            timezone,
            self,
        );
        // will decrease the date until `time_in_timezoned_day` becomes greater than `max_seconds_since_timezoned_day_start`
        let mut backward_iter = BackwardDecompose::new(
            seconds_since_dataset_start,
            max_seconds_since_timezoned_day_start,
            timezone,
            self,
        );
        // we want to skip the first date as it is already provided by `forward_iter`
        backward_iter.next();

        forward_iter.chain(backward_iter)
    }

    pub fn decompositions_utc<'a>(
        &'a self,
        seconds_since_dataset_start: &SecondsSinceDatasetUTCStart,
    ) -> impl Iterator<Item = (DaysSinceDatasetStart, SecondsSinceUTCDayStart)> + 'a {
        DecomposeUtc::new(seconds_since_dataset_start, self)
    }

    pub fn compose_utc(
        &self,
        day: &DaysSinceDatasetStart,
        seconds_in_day: &SecondsSinceUTCDayStart,
    ) -> SecondsSinceDatasetUTCStart {
        debug_assert!(day.days < self.nb_of_days());

        let seconds_i32 = i32::from(day.days) * SECONDS_IN_A_DAY
            + seconds_in_day.seconds
            + MAX_SECONDS_IN_UTC_DAY;

        // seconds_i32 should be >=0 since
        //  - i32::from(day.days) * SECONDS_PER_DAY >= 0
        //         since day.days >= 0 and SECONDS_PER_DAY > 0
        //  - seconds_in_day.seconds + MAX_SECONDS_IN_UTC_DAY >= 0
        //        since seconds_in_day.seconds >= - MAX_SECONDS_IN_UTC_DAY by construction on SecondsSinceUTCDayStart
        // which is exactly how first_datetime() is constructed
        debug_assert!(seconds_i32 >= 0);

        let seconds_u32 = seconds_i32 as u32;

        SecondsSinceDatasetUTCStart {
            seconds: seconds_u32,
        }
    }

    pub fn compose(
        &self,
        day: &DaysSinceDatasetStart,
        seconds_in_day: &SecondsSinceTimezonedDayStart,
        timezone: &Timezone,
    ) -> SecondsSinceDatasetUTCStart {
        debug_assert!(day.days < self.nb_of_days());
        let date = *self.first_date() + chrono::Duration::days(day.days as i64);
        // Since DaySinceDatasetStart can only be constructed from the calendar, the date should be allowed by the calendar
        debug_assert!(self.contains_date(&date));
        use chrono::offset::TimeZone;
        let datetime_timezoned = timezone.from_utc_date(&date).and_hms(12, 0, 0)
            - chrono::Duration::hours(12)
            + chrono::Duration::seconds(seconds_in_day.seconds as i64);
        use chrono_tz::UTC;
        let datetime_utc = datetime_timezoned.with_timezone(&UTC).naive_utc();

        debug_assert!(self.contains_datetime(&datetime_utc));
        let seconds_i64 = (datetime_utc - self.first_datetime()).num_seconds();
        // seconds_i64 should be >=0 since
        // by construction above, we have
        // datetime_utc >=
        //        first_date
        //        - MAX_SECONDS_SINCE_TIMEZONED_DAY_START
        //        - MAX_TIMEZONE_OFFSET
        // which is exactly how first_datetime() is constructed
        debug_assert!(seconds_i64 >= 0);
        // seconds_i64 <= u32::MAX since :
        // - day < MAX_DAYS_IN_CALENDAR
        // - seconds_in_day < MAX_SECONDS_IN_TIMEZONED_DAY
        // thus seconds_i64 <=  MAX_DAYS_IN_CALENDAR * 24 * 60 * 60
        //                     + MAX_SECONDS_IN_TIMEZONED_DAY
        //                     + MAX_TIMEZONE_OFFSET
        // and the latter number is smaller than u32::MAX
        static_assertions::const_assert!(
            (MAX_DAYS_IN_CALENDAR as i64) * (SECONDS_IN_A_DAY as i64)
                + (MAX_SECONDS_IN_TIMEZONED_DAY as i64)
                + (MAX_TIMEZONE_OFFSET as i64)
                <= u32::MAX as i64
        );
        debug_assert!(seconds_i64 <= u32::MAX as i64);
        let seconds_u32 = seconds_i64 as u32;
        SecondsSinceDatasetUTCStart {
            seconds: seconds_u32,
        }
    }

    /// Returns Some(_) if datetime is allowed by calendar,
    /// and None otherwise
    pub fn from_naive_datetime(
        &self,
        datetime: &chrono::NaiveDateTime,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        if !self.contains_datetime(datetime) {
            return None;
        }
        let seconds_i64 = (*datetime - self.first_datetime()).num_seconds();
        // seconds_i64 should be >=0 since calendar.contains_datetime(&datetime)
        debug_assert!(seconds_i64 >= 0);
        // seconds_i64 <= u32::MAX
        // since calendar.contains_datetime(&datetime) and
        // (last_datetime - first_datetime).num_seconds() <=
        //   MAX_DAYS_IN_CALENDAR * 24 * 60 * 60
        //    + MAX_SECONDS_IN_TIMEZONED_DAY
        //    + MAX_TIMEZONE_OFFSET
        // and the latter number is smaller than u32::MAX
        static_assertions::const_assert!(
            (MAX_DAYS_IN_CALENDAR as i64) * (SECONDS_IN_A_DAY as i64)
                + (MAX_SECONDS_IN_TIMEZONED_DAY as i64)
                + (MAX_TIMEZONE_OFFSET as i64)
                <= u32::MAX as i64
        );
        debug_assert!(seconds_i64 <= u32::MAX as i64);

        let seconds_u32 = seconds_i64 as u32;
        let result = SecondsSinceDatasetUTCStart {
            seconds: seconds_u32,
        };
        Some(result)
    }

    fn make_day_unchecked(&self, days_offset: u16) -> DaysSinceDatasetStart {
        debug_assert!(days_offset < self.nb_of_days());
        DaysSinceDatasetStart { days: days_offset }
    }
}

pub struct DaysIter {
    inner: std::ops::Range<u16>,
}

impl Iterator for DaysIter {
    type Item = DaysSinceDatasetStart;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|idx| DaysSinceDatasetStart { days: idx })
    }
}

// Iterator that provides decompositions of `datetime_timezoned_to_decompose`
// in the form of  a `(day, time_in_timezoned_day)`.
// It will advance the `day` until `time_in_timezoned_day`
// becomes strictly smaller than `min_seconds_since_timezoned_day_start`
pub struct ForwardDecompose<'calendar> {
    datetime_timezoned_to_decompose: chrono::DateTime<Timezone>,
    has_reference_date: Option<chrono::Date<Timezone>>,
    min_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
    calendar: &'calendar Calendar,
}

impl<'calendar> ForwardDecompose<'calendar> {
    fn new(
        seconds_since_dataset_start: &SecondsSinceDatasetUTCStart,
        min_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
        timezone: &Timezone,
        calendar: &'calendar Calendar,
    ) -> Self {
        let datetime_utc = calendar.first_datetime()
            + chrono::Duration::seconds(i64::from(seconds_since_dataset_start.seconds));
        debug_assert!(calendar.contains_datetime(&datetime_utc));
        use chrono::TimeZone;
        let datetime_timezoned = timezone.from_utc_datetime(&datetime_utc);
        let date_timezoned = datetime_timezoned.date();
        Self {
            datetime_timezoned_to_decompose: datetime_timezoned,
            has_reference_date: Some(date_timezoned),
            min_seconds_since_timezoned_day_start,
            calendar,
        }
    }
}

impl<'calendar> Iterator for ForwardDecompose<'calendar> {
    type Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref reference_date) = self.has_reference_date {
            let has_decomposition = self
                .calendar
                .decompose(self.datetime_timezoned_to_decompose, reference_date);
            if let Some((day, seconds)) = has_decomposition {
                if seconds >= self.min_seconds_since_timezoned_day_start {
                    self.has_reference_date = reference_date.succ_opt();
                    return Some((day, seconds));
                }
            }
        }
        None
    }
}

pub struct DecomposeUtc<'calendar> {
    canonical_day: i32,
    canonical_time_in_day: i32,
    iter: std::ops::Range<i32>,
    // not really useful, but allows debug_asserting  that this iterator
    // does not create a DaySinceDatasetStart outside of the range allowed by the calendar
    calendar: &'calendar Calendar,
}

impl<'calendar> DecomposeUtc<'calendar> {
    fn new(
        seconds_since_dataset_start: &SecondsSinceDatasetUTCStart,
        calendar: &'calendar Calendar,
    ) -> Self {
        // 0
        //
        // [------------|----|----| ... |----|----|------------]
        //

        let first_day_at_midnight_i64 = i64::from(MAX_SECONDS_IN_UTC_DAY);
        let last_day_at_midnight_i64 = first_day_at_midnight_i64
            + i64::from(calendar.last_day_offset) * i64::from(SECONDS_IN_A_DAY);
        let (canonical_day, canonical_time_in_day): (u16, i32) = {
            let datetime_i64 = i64::from(seconds_since_dataset_start.seconds);
            if datetime_i64 <= first_day_at_midnight_i64 {
                let day = 0u16;
                let time_in_day_i64 = datetime_i64 - first_day_at_midnight_i64;
                // cast to i32 is safe because :
                //   * time_in_day_i64 <= 0 since
                //         datetime_i64 <= first_day_at_midnight_i64
                //   * time_in_day_i64 >= - first_day_at_midnight_i64 >= - i32::MAX since
                //        datetime_i64 >=0
                let time_in_day_i32 = time_in_day_i64 as i32;
                (day, time_in_day_i32)
            } else if datetime_i64 >= last_day_at_midnight_i64 {
                let day = calendar.last_day_offset;
                let time_in_day_i64 = datetime_i64 - last_day_at_midnight_i64;
                // cast to i32 is safe because :
                //   * time_in_day_i64 <= SECONDS_IN_UTC_DAY < i32::MAX since
                //         datetime_i64 <= last_day_at_midnight_i64 + SECONDS_IN_UTC_DAY
                //         by construction of SecondsSinceDatasetUTCStart
                //   * time_in_day_i64 >= 0 since
                //         datetime_i64 >= last_day_at_midnight_i64
                let time_in_day_i32 = time_in_day_i64 as i32;
                (day, time_in_day_i32)
            } else {
                // first_day_at_midnight_i64 < datetime_i64 < last_day_at_midnight_i64
                let day_i64 =
                    (datetime_i64 - first_day_at_midnight_i64) / i64::from(SECONDS_IN_A_DAY);
                let time_in_day_i64 =
                    (datetime_i64 - first_day_at_midnight_i64) % i64::from(SECONDS_IN_A_DAY);

                // cast to u16 is safe because
                //  * day_i64 >= 0 since
                //          datetime_i64 - first_day_at_midnight_i64 >= 0
                //  * day_i64 <= calendar.last_day_offset < MAX_DAYS_IN_CALENDAR < u16::MAX since
                //          (datetime_i64 - first_day_at_midnight_i64) <= (last_day_at_midnight_i64 - first_day_at_midnight_i64)
                //                                                     <= calendar.last_day_offset * SECONDS_IN_A_DAY
                let day = day_i64 as u16;

                // cast to i32 is safe because
                //  |time_in_day_i64| <= SECONDS_IN_A_DAY <= i32::MAX
                let time_in_day_i32 = time_in_day_i64 as i32;
                (day, time_in_day_i32)
            }
        };

        // we are going to generate pairs
        // (canonical_day + k, canonical_time_in_day - k * SECONDS_IN_A_DAY)
        // where k covers all integers such that
        //   0 <= canonical_day + k <= calendar.last_day_offset
        //   - MAX_SECONDS_IN_UTC_DAY <= canonical_time_in_day - k * SECONDS_IN_A_DAY <= MAX_SECONDS_IN_UTC_DAY
        // so k must satisfies
        //   k >= - canonical_day
        //   k >= (canonical_time_in_day - MAX_SECONDS_IN_UTC_DAY) / SECONDS_IN_A_DAY
        // and
        //   k <= calendar.last_day_offset - canonical_day
        //   k <= (canonical_time_in_day + MAX_SECONDS_IN_UTC_DAY) / SECONDS_IN_A_DAY

        let canonical_day_i32 = i32::from(canonical_day);

        let k_lower_bound_from_time_in_day = {
            let div = (canonical_time_in_day - MAX_SECONDS_IN_UTC_DAY) / SECONDS_IN_A_DAY;
            let rem = (canonical_time_in_day - MAX_SECONDS_IN_UTC_DAY) % SECONDS_IN_A_DAY;
            if rem > 0 {
                div + 1
            } else {
                div
            }
        };

        let k_min = std::cmp::max(-canonical_day_i32, k_lower_bound_from_time_in_day);

        let k_upper_bound_from_time_in_day = {
            let div = (canonical_time_in_day + MAX_SECONDS_IN_UTC_DAY) / SECONDS_IN_A_DAY;
            let rem = (canonical_time_in_day + MAX_SECONDS_IN_UTC_DAY) % SECONDS_IN_A_DAY;
            if rem < 0 {
                div - 1
            } else {
                div
            }
        };

        let k_max = std::cmp::min(
            i32::from(calendar.last_day_offset) - canonical_day_i32,
            k_upper_bound_from_time_in_day,
        );

        Self {
            canonical_day: canonical_day_i32,
            canonical_time_in_day,
            iter: k_min..(k_max + 1),
            calendar,
        }
    }
}

impl<'calendar> Iterator for DecomposeUtc<'calendar> {
    type Item = (DaysSinceDatasetStart, SecondsSinceUTCDayStart);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|k| {
            let day_i32 = self.canonical_day + k;

            // here we make unsafe things, but everything
            // is safe because we were extra careful in Self::new()

            let day_u16 = day_i32 as u16;
            let day = self.calendar.make_day_unchecked(day_u16);

            let time_in_day = SecondsSinceUTCDayStart::new_unchecked(
                self.canonical_time_in_day - SECONDS_IN_A_DAY * k,
            );
            (day, time_in_day)
        })
    }
}

// Iterator that provides decompositions of `datetime_timezoned_to_decompose`
// in the form of  a `(day, time_in_timezoned_day)`.
// It will decrease the `day` until `time_in_timezoned_day`
// becomes strictly greater than `max_seconds_since_timezoned_day_start`
pub struct BackwardDecompose<'calendar> {
    datetime_timezoned_to_decompose: chrono::DateTime<Timezone>,
    has_reference_date: Option<chrono::Date<Timezone>>,
    max_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
    calendar: &'calendar Calendar,
}

impl<'calendar> BackwardDecompose<'calendar> {
    fn new(
        seconds_since_dataset_start: &SecondsSinceDatasetUTCStart,
        max_seconds_since_timezoned_day_start: SecondsSinceTimezonedDayStart,
        timezone: &Timezone,
        calendar: &'calendar Calendar,
    ) -> Self {
        let datetime_utc = calendar.first_datetime()
            + chrono::Duration::seconds(i64::from(seconds_since_dataset_start.seconds));
        debug_assert!(calendar.contains_datetime(&datetime_utc));
        use chrono::TimeZone;
        let datetime_timezoned = timezone.from_utc_datetime(&datetime_utc);
        let date_timezoned = datetime_timezoned.date();
        Self {
            datetime_timezoned_to_decompose: datetime_timezoned,
            has_reference_date: Some(date_timezoned),
            max_seconds_since_timezoned_day_start,
            calendar,
        }
    }
}

impl<'calendar> Iterator for BackwardDecompose<'calendar> {
    type Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref reference_date) = self.has_reference_date {
            let has_decomposition = self
                .calendar
                .decompose(self.datetime_timezoned_to_decompose, reference_date);
            if let Some((day, seconds)) = has_decomposition {
                if seconds <= self.max_seconds_since_timezoned_day_start {
                    self.has_reference_date = reference_date.pred_opt();
                    return Some((day, seconds));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    const SECONDS_PER_HOUR: i32 = 60 * 60;

    use super::Calendar;
    #[test]
    fn test_decompositions_utc() {
        let calendar = Calendar::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveDate::from_ymd(2020, 1, 30),
        );

        {
            // Jan 1st at 12:00
            let datetime = NaiveDate::from_ymd(2020, 1, 1).and_hms(12, 0, 0);
            let seconds_since_dataset_start = calendar.from_naive_datetime(&datetime).unwrap();

            let decompositions: Vec<_> = calendar
                .decompositions_utc(&seconds_since_dataset_start)
                .collect();

            // Jan 1st + 12h
            assert_eq!(decompositions[0].0.days, 0);
            assert_eq!(decompositions[0].1.seconds, 12 * SECONDS_PER_HOUR);

            // Jan 2nd - 12h
            assert_eq!(decompositions[1].0.days, 1);
            assert_eq!(decompositions[1].1.seconds, -12 * SECONDS_PER_HOUR);

            // Jan 3rd - 36h
            assert_eq!(decompositions[2].0.days, 2);
            assert_eq!(decompositions[2].1.seconds, -36 * SECONDS_PER_HOUR);

            // Jan 4th - 60h
            assert_eq!(decompositions[3].0.days, 3);
            assert_eq!(decompositions[3].1.seconds, -60 * SECONDS_PER_HOUR);

            assert_eq!(decompositions.len(), 4);
        }

        {
            // Jan 10st at 12:00
            let datetime = NaiveDate::from_ymd(2020, 1, 10).and_hms(12, 0, 0);
            let seconds_since_dataset_start = calendar.from_naive_datetime(&datetime).unwrap();

            let decompositions: Vec<_> = calendar
                .decompositions_utc(&seconds_since_dataset_start)
                .collect();

            // Jan 8 at +48h + 12h
            assert_eq!(decompositions[0].0.days, 7);
            assert_eq!(decompositions[0].1.seconds, (48 + 12) * SECONDS_PER_HOUR);

            // Jan 9 at +24h +12h
            assert_eq!(decompositions[1].0.days, 8);
            assert_eq!(decompositions[1].1.seconds, (24 + 12) * SECONDS_PER_HOUR);

            // Jan 10 at +12h
            assert_eq!(decompositions[2].0.days, 9);
            assert_eq!(decompositions[2].1.seconds, 12 * SECONDS_PER_HOUR);

            // Jan 11 at -12h
            assert_eq!(decompositions[3].0.days, 10);
            assert_eq!(decompositions[3].1.seconds, -12 * SECONDS_PER_HOUR);

            // Jan 12 at -24h -12h
            assert_eq!(decompositions[4].0.days, 11);
            assert_eq!(decompositions[4].1.seconds, (-24 - 12) * SECONDS_PER_HOUR);

            // Jan 13 at -48h -12h
            assert_eq!(decompositions[5].0.days, 12);
            assert_eq!(decompositions[5].1.seconds, (-48 - 12) * SECONDS_PER_HOUR);

            assert_eq!(decompositions.len(), 6);
        }

        {
            // Jan 30st at 12:00
            let datetime = NaiveDate::from_ymd(2020, 1, 30).and_hms(12, 0, 0);
            let seconds_since_dataset_start = calendar.from_naive_datetime(&datetime).unwrap();

            let decompositions: Vec<_> = calendar
                .decompositions_utc(&seconds_since_dataset_start)
                .collect();

            // Jan 28 at +48h + 12h
            assert_eq!(decompositions[0].0.days, 27);
            assert_eq!(decompositions[0].1.seconds, (48 + 12) * SECONDS_PER_HOUR);

            // Jan 29 at +24h +12h
            assert_eq!(decompositions[1].0.days, 28);
            assert_eq!(decompositions[1].1.seconds, (24 + 12) * SECONDS_PER_HOUR);

            // Jan 30 at +12h
            assert_eq!(decompositions[2].0.days, 29);
            assert_eq!(decompositions[2].1.seconds, 12 * SECONDS_PER_HOUR);

            assert_eq!(decompositions.len(), 3);
        }

        {
            // Jan 31st at 12:00
            let datetime = NaiveDate::from_ymd(2020, 1, 31).and_hms(12, 0, 0);
            let seconds_since_dataset_start = calendar.from_naive_datetime(&datetime).unwrap();

            let decompositions: Vec<_> = calendar
                .decompositions_utc(&seconds_since_dataset_start)
                .collect();

            // Jan 28 at +48h + 12h
            assert_eq!(decompositions[0].0.days, 28);
            assert_eq!(decompositions[0].1.seconds, (48 + 12) * SECONDS_PER_HOUR);

            // Jan 30 at +24h +12h
            assert_eq!(decompositions[1].0.days, 29);
            assert_eq!(decompositions[1].1.seconds, (24 + 12) * SECONDS_PER_HOUR);

            assert_eq!(decompositions.len(), 2);
        }

        {
            // Dec 31st at 12:00
            let datetime = NaiveDate::from_ymd(2019, 12, 31).and_hms(12, 0, 0);
            let seconds_since_dataset_start = calendar.from_naive_datetime(&datetime).unwrap();

            let decompositions: Vec<_> = calendar
                .decompositions_utc(&seconds_since_dataset_start)
                .collect();

            // Jan 1st - 12h
            assert_eq!(decompositions[0].0.days, 0);
            assert_eq!(decompositions[0].1.seconds, -12 * SECONDS_PER_HOUR);
        }
    }
}
