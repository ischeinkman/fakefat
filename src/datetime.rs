#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Date {
    year: u16,
    month: u8,
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
    pub fn with_year(self, year: u16) -> Date {
        debug_assert!(year >= 1980);
        Date { year, ..self }
    }
    pub fn with_month(self, month: u8) -> Date {
        debug_assert!(month <= 12 && month > 0);
        Date { month, ..self }
    }
    pub fn with_day(self, day: u8) -> Date {
        debug_assert!(day <= 31 && day > 0, "Bad day: {:?}", day);
        Date { day, ..self }
    }

    pub fn year(self) -> u16 {
        self.year
    }
    pub fn month(self) -> u8 {
        self.month
    }
    pub fn day(self) -> u8 {
        self.day
    }

    pub fn fat_encode(self) -> u16 {
        let epoch_year = self.year - 1980;
        let year_part = epoch_year << 9;

        let month_part = (self.month as u16) << 5;

        let day_part = self.day as u16;

        year_part | month_part | day_part
    }

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
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    tenths: u8,
}

impl Time {
    pub fn with_hour(self, hour: u8) -> Time {
        debug_assert!(hour <= 23);
        Time { hour, ..self }
    }
    pub fn with_minute(self, minute: u8) -> Time {
        debug_assert!(minute <= 59);
        Time { minute, ..self }
    }
    pub fn with_second(self, second: u8) -> Time {
        debug_assert!(second <= 59);
        Time { second, ..self }
    }
    pub fn with_tenths(self, tenths: u8) -> Time {
        debug_assert!(tenths < 10);
        Time { tenths, ..self }
    }

    pub fn hour(self) -> u8 {
        self.hour
    }
    pub fn minute(self) -> u8 {
        self.minute
    }
    pub fn second(self) -> u8 {
        self.second
    }

    pub fn decode(encoded: u16) -> Self {
        let hour = (encoded >> 11) as u8;
        let min = ((encoded >> 5) & 0x3F) as u8;
        let sec = ((encoded & 0x1F) * 2) as u8;
        Time::default()
            .with_hour(hour)
            .with_minute(min)
            .with_second(sec)
    }
    pub fn with_hi_res(mut self, hi_res_info: u8) -> Self {
        debug_assert!((hi_res_info <= 9) || (hi_res_info >= 100 && hi_res_info <= 109));
        self.second += hi_res_info / 100;
        self.tenths = hi_res_info % 100;
        self
    }

    pub fn fat_encode_simple(self) -> u16 {
        let hour_part = (self.hour as u16) << 11;
        let min_part = (self.minute as u16) << 5;
        let sec_part = (self.second / 2) as u16;
        hour_part | min_part | sec_part
    }
    pub fn fat_encode_hi_res(self) -> u8 {
        let second_mod_part = (self.second % 2) * 100;
        second_mod_part | self.tenths
    }
}
