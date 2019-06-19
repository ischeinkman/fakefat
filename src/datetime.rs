const NONLEAP_MONTH_RANGES: [u16; 13] = [
    0,
    31,
    31 + 28,
    31 + 28 + 31,
    31 + 28 + 31 + 30,
    31 + 28 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
    31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30 + 31,
];
const LEAP_MONTH_RANGES: [u16; 13] = [
    0,
    31,
    31 + 29,
    31 + 29 + 31,
    31 + 29 + 31 + 30,
    31 + 29 + 31 + 30 + 31,
    31 + 29 + 31 + 30 + 31 + 30,
    31 + 29 + 31 + 30 + 31 + 30 + 31,
    31 + 29 + 31 + 30 + 31 + 30 + 31 + 31,
    31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
    31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
    31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
    31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30 + 31,
];

/// Represents a standard Gregorian date.
///
/// Note that while technically the struct would seem to be compatible with
/// dates pre-unix epoch, they are still considered incompatible.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Date {
    /// Year AD.
    year: u16,

    /// Month, with January = 1.
    month: u8,

    /// Day of the month, from 1 - 31.
    day: u8,
}

impl Default for Date {
    fn default() -> Self {
        Date {
            year: 1980,
            month: 1,
            day: 1,
        }
    }
}

impl Date {
    /// Constructs a new `Date` out of `self`'s month and day combined with the
    /// passed `year` value.
    pub fn with_year(self, year: u16) -> Date {
        debug_assert!(year >= 1980);
        Date { year, ..self }
    }

    /// Constructs a new `Date` out of `self`'s year and day combined with the
    /// passed `month` value.
    pub fn with_month(self, month: u8) -> Date {
        debug_assert!(month <= 12 && month > 0);
        Date { month, ..self }
    }

    /// Constructs a new `Date` out of `self`'s year and month combined with the
    /// passed `day` value.
    pub fn with_day(self, day: u8) -> Date {
        debug_assert!(day <= 31 && day > 0, "Bad day: {:?}", day);
        Date { day, ..self }
    }

    /// Year AD.
    pub fn year(self) -> u16 {
        self.year
    }

    /// Month of the year, with January = 1.
    pub fn month(self) -> u8 {
        self.month
    }

    /// Day of the month, from 1 - 31.
    pub fn day(self) -> u8 {
        self.day
    }

    /// Converts a human-readable date into a FAT filesystem compatible format.
    pub fn fat_encode(self) -> u16 {
        let epoch_year = self.year - 1980;
        let year_part = epoch_year << 9;

        let month_part = u16::from(self.month) << 5;

        let day_part = u16::from(self.day);

        year_part | month_part | day_part
    }

    /// Converts a FAT filesystem-encoded date into a human readable format.
    pub fn fat_decode(encoded: u16) -> Date {
        let epoch_year = encoded >> 9;
        let year = epoch_year + 1980;

        let month = ((encoded >> 5) & 0xF) as u8;
        let day = (encoded & 0x1f) as u8;

        Date::default()
            .with_year(year)
            .with_month(month)
            .with_day(day)
    }

    /// Extracts the date from the number of milliseconds since the Unix Epoch.
    pub fn from_epoch_millis(millis: u64) -> Date {
        let days_since_epoch = millis / (24 * 60 * 60 * 1000);
        let unleaped_years_since_epoch = days_since_epoch / 365;
        let leap_years = unleaped_years_since_epoch / 4;
        let raw_year_offset = ((days_since_epoch as i32) % 365i32) - (leap_years as i32);
        debug_assert!(
            raw_year_offset < 365 && raw_year_offset > -365,
            "Bad raw: {}",
            raw_year_offset
        );
        let (years, year_offset) = if raw_year_offset < 0 {
            (
                (unleaped_years_since_epoch - 1) as u16,
                (raw_year_offset + 365) as u16,
            )
        } else {
            (unleaped_years_since_epoch as u16, raw_year_offset as u16)
        };
        let month_ranges = if years % 4 == 0 {
            LEAP_MONTH_RANGES
        } else {
            NONLEAP_MONTH_RANGES
        };
        let mut month = 0;
        let mut day = 0;
        for idx in 0..13 {
            if year_offset < month_ranges[idx] {
                month = idx;
                day = if idx == 0 {
                    year_offset + 1
                } else {
                    year_offset - month_ranges[idx - 1] + 1
                };
                break;
            }
        }
        Date::default()
            .with_day(day as u8)
            .with_month(month as u8)
            .with_year(1970 + years)
    }
}

/// Represents a standard time in 24 hour format with precision up to 0.1 second.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    tenths: u8,
}

impl Time {

    /// Constructs a copy of `self` with the hour set to `hour`. 
    pub fn with_hour(self, hour: u8) -> Time {
        debug_assert!(hour <= 23);
        Time { hour, ..self }
    }
    
    /// Constructs a copy of `self` with the minute set to `minute`. 
    pub fn with_minute(self, minute: u8) -> Time {
        debug_assert!(minute <= 59);
        Time { minute, ..self }
    }
    /// Constructs a copy of `self` with the second set to `second`. 
    pub fn with_second(self, second: u8) -> Time {
        debug_assert!(second <= 59);
        Time { second, ..self }
    }
    /// Constructs a copy of `self` with the tenths of second field set to `tenths`. 
    pub fn with_tenths(self, tenths: u8) -> Time {
        debug_assert!(tenths < 10);
        Time { tenths, ..self }
    }

    /// The whole hours since midnight, between 0 and 23 inclusive. 
    pub fn hour(self) -> u8 {
        self.hour
    }

    /// The minute passed the hour, between 0 and 59 inclusive. 
    pub fn minute(self) -> u8 {
        self.minute
    }

    /// The seconds since the minute, between 0 and 59 inclusive. 
    pub fn second(self) -> u8 {
        self.second
    }

    /// Decodes a low-precision FAT-encoded clock time into a `Time` value. 
    /// 
    /// Due to FAT precision limitations, this means that the resulting `second()`
    /// will always be even and the resulting `tenths()` will always be 0. 
    pub fn decode(encoded: u16) -> Self {
        let hour = (encoded >> 11) as u8;
        let min = ((encoded >> 5) & 0x3F) as u8;
        let sec = ((encoded & 0x1F) * 2) as u8;
        Time::default()
            .with_hour(hour)
            .with_minute(min)
            .with_second(sec)
    }

    /// Constructs a copy of `self` modified to include the information included
    /// in the supplied FAT32 high-resolution-time byte. 
    /// 
    /// This byte includes information for both tenths of a second and for `self.second % 2`. 
    pub fn with_hi_res(mut self, hi_res_info: u8) -> Self {
        debug_assert!((hi_res_info <= 9) || (hi_res_info >= 100 && hi_res_info <= 109));
        self.second -= self.second % 2;
        self.second += hi_res_info / 100;
        self.tenths = hi_res_info % 100;
        self
    }

    /// Encodes the standard portion of this time's FAT filesystem-encoded
    /// representation.
    pub fn fat_encode_simple(self) -> u16 {
        let hour_part = u16::from(self.hour) << 11;
        let min_part = u16::from(self.minute) << 5;
        let sec_part = u16::from(self.second / 2);
        hour_part | min_part | sec_part
    }

    /// Encodes the high resolution portion of this time's FAT filesystem-encoded
    /// representation.
    pub fn fat_encode_hi_res(self) -> u8 {
        let second_mod_part = (self.second % 2) * 100;
        second_mod_part | self.tenths
    }

    /// Extracts the time from the number of milliseconds since the Unix Epoch.
    pub fn from_epoch_millis(millis_since_epoch: u64) -> Time {
        let secs_since_epoch = millis_since_epoch / 1000;
        let time_part = secs_since_epoch % (24 * 60 * 60);
        let hour = (time_part / 3600) as u8;
        let minute = ((time_part / 60) % 60) as u8;
        let second = (time_part % 60) as u8;
        let tenths = ((millis_since_epoch % 1000) / 100) as u8;

        Time::default()
            .with_hour(hour)
            .with_minute(minute)
            .with_second(second)
            .with_tenths(tenths)
    }
}
