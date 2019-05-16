use crate::datetime::{Date, Time};
use crate::traits::{DirEntryOps, DirectoryOps, FileMetadata, FileOps, FileSystemOps};

use std::fs::{self, DirEntry, File, Metadata};
use std::io::{self, Read, Seek};
use std::path::{PathBuf};
use std::time::{SystemTime};

impl FileOps for File {
    fn read_at(&mut self, offset: usize, buffer: &mut [u8]) -> usize {
        self.seek(io::SeekFrom::Start(offset as u64)).unwrap();
        self.read(buffer).unwrap()
    }
}

impl DirEntryOps for DirEntry {
    type NameType = String;
    fn name(&self) -> String {
        self.file_name().into_string().unwrap()
    }
    fn meta(&self) -> FileMetadata {
        self.metadata().map(get_metadata).unwrap()
    }
}

impl DirectoryOps for PathBuf {
    type EntryType = DirEntry;
    fn entries(&self) -> Vec<DirEntry> {
        fs::read_dir(&self)
            .map(|iter| iter.map(Result::unwrap).collect())
            .unwrap()
    }
}

pub struct StdFileSystem {}

impl FileSystemOps for StdFileSystem {
    type DirEntryType = DirEntry;
    type DirectoryType = PathBuf;
    type FileType = File;

    fn get_file(&mut self, path: &str) -> Option<File> {
        let raw = File::open(path);
        match raw {
            Ok(f) => Some(f),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => None,
                _ => {
                    Result::<(), io::Error>::Err(e).unwrap();
                    panic!();
                }
            },
        }
    }
    fn get_dir(&mut self, path: &str) -> Option<PathBuf> {
        //println!("{}", path);
        let retval = PathBuf::from(path);
        let dir_read_res = fs::read_dir(path);
        match dir_read_res {
            Ok(_) => Some(retval),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => None,
                _ => {
                    Result::<(), io::Error>::Err(e).unwrap();
                    panic!();
                }
            },
        }
    }
}

fn get_metadata(mt: Metadata) -> FileMetadata {
    let (cdate, ctime) = mt.created().map(sys_time_to_date_time).unwrap_or_default();
    let (mdate, mtime) = mt.modified().map(sys_time_to_date_time).unwrap_or_default();
    let (adate, _) = mt.accessed().map(sys_time_to_date_time).unwrap_or_default();
    let size = if mt.is_file() { mt.len() as u32 } else { 0 };
    let is_read_only = mt.permissions().readonly();
    let is_directory = mt.is_dir();
    let is_hidden = false; //TODO: Check for dot start?
    FileMetadata {
        is_directory,
        is_hidden,
        is_read_only,
        create_date: cdate,
        create_time: ctime,
        access_date: adate,
        modify_time: mtime,
        modify_date: mdate,
        size,
    }
}

fn sys_time_to_date_time(sys: SystemTime) -> (Date, Time) {
    let millis_since_epoch = sys
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    (
        extract_date_from_epoch_millis(millis_since_epoch),
        extract_time_from_epoch_millis(millis_since_epoch),
    )
}

fn extract_time_from_epoch_millis(millis_since_epoch: u64) -> Time {
    let secs_since_epoch = millis_since_epoch / 1000;
    let time_part = secs_since_epoch % (24 * 60 * 60);
    let hour = (time_part / 3600) as u8;
    let minute = ((time_part / 60) % 60) as u8;
    let second = (time_part % 60) as u8;
    let tenths = ((millis_since_epoch % 1000) / 100) as u8;

    let time = Time::default()
        .with_hour(hour)
        .with_minute(minute)
        .with_second(second)
        .with_tenths(tenths);
    time
}

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

fn extract_date_from_epoch_millis(millis: u64) -> Date {
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
