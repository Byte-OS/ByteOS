#![no_std]

use core::fmt::Display;

/// DateTime struct
#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    /// Timestamp
    pub timestamp: usize,
    /// year
    pub year: u16,
    /// month of the year
    pub month: u8,
    /// day of the month
    pub day: u8,
    /// hour of the day
    pub hour: u8,
    /// minutes of the hour
    pub minutes: u8,
    /// seconds of the minute
    pub seconds: u8,
}

const SECONDS_PER_DAY: usize = 24 * 60 * 60;
const SECONDS_PER_YEAR: usize = SECONDS_PER_DAY * 365;
const SECONDS_PER_HOUR: usize = 60 * 60;

const fn seconds_year(year: u16) -> usize {
    if is_leap_year(year) {
        SECONDS_PER_YEAR + SECONDS_PER_DAY
    } else {
        SECONDS_PER_YEAR
    }
}

const fn is_leap_year(year: u16) -> bool {
    year % 400 == 0 || (year % 4 == 0 && year % 100 != 0)
}

const fn seconds_month(month: u8, year: u16) -> usize {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => SECONDS_PER_DAY * 31,
        4 | 6 | 9 | 11 => SECONDS_PER_DAY * 30,
        2 => SECONDS_PER_DAY * (28 + is_leap_year(year) as usize),
        _ => unreachable!(),
    }
}

impl DateTime {
    pub const fn new(timestamp: usize) -> Self {
        // 上海时间 UTC + 8
        let mut ts = timestamp + SECONDS_PER_HOUR * 8;
        let mut year = 1970;
        while ts >= seconds_year(year) {
            ts -= seconds_year(year);
            year += 1;
        }

        let mut month = 1;

        while ts >= seconds_month(month, year) {
            ts -= seconds_month(month, year);
            month += 1;
        }

        let day = (ts / SECONDS_PER_DAY) as u8 + 1;
        ts %= SECONDS_PER_DAY;
        let hour = (ts / SECONDS_PER_HOUR) as u8;
        ts %= SECONDS_PER_HOUR;
        let minutes = (ts / 60) as u8;
        let seconds = (ts % 60) as u8;

        Self {
            timestamp,
            year: year,
            month,
            day,
            hour,
            minutes,
            seconds,
        }
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}年{:02}月{:02}日 {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minutes, self.seconds
        ))
    }
}
