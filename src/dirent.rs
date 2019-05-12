use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::ops::{BitAnd};
use crate::shortname::ShortName;
use crate::datetime::{Date, Time};

#[derive(Clone, Debug, Default, Copy)]
pub struct DirFileEntryData {
    name: ShortName,
    attrs: FileAttributes,
    create_time : Time,
    create_date: Date,
    access_date: Date,
    first_cluster: u32,
    modify_time: Time,
    modify_date: Date,
    size: u32,

    read_idx : usize,
}

impl DirFileEntryData {
    fn read_byte(&self, idx : usize) -> u8 {
        match idx {
            b @ 0..=10 => self.name.read_byte(b),
            11 => self.attrs.0, 
            12 => self.name.case_flag(), 
            13 => self.create_time.fat_encode_hi_res(), 
            14 => (self.create_time.fat_encode_simple() & 0xFF) as u8, 
            15 => ((self.create_time.fat_encode_simple() >> 8) & 0xFF) as u8, 
            16 => (self.create_date.fat_encode() & 0xFF) as u8, 
            17 => ((self.create_date.fat_encode() >> 8) & 0xFF) as u8, 
            18 => (self.access_date.fat_encode() & 0xFF) as u8, 
            19 => ((self.access_date.fat_encode() >> 8) & 0xFF) as u8, 
            20 => ((self.first_cluster >> 16) & 0xFF) as u8, 
            21 => ((self.first_cluster >> 24) & 0xFF) as u8, 
            22 => (self.modify_time.fat_encode_simple() & 0xFF) as u8, 
            23 => ((self.modify_time.fat_encode_simple() >> 8) & 0xFF) as u8, 
            24 => (self.modify_date.fat_encode() & 0xFF) as u8, 
            25 => ((self.modify_date.fat_encode() >> 8) & 0xFF) as u8, 
            26 => ((self.first_cluster) & 0xFF) as u8, 
            27 => ((self.first_cluster >> 8) & 0xFF) as u8, 
            28 => ((self.size) & 0xFF) as u8, 
            29 => ((self.size >> 8) & 0xFF) as u8, 
            30 => ((self.size >> 16) & 0xFF) as u8, 
            31 => ((self.size >> 24) & 0xFF) as u8, 
            _ => 0,
        }
    }
}
impl Read for DirFileEntryData {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let mut offset = 0;
        while offset + self.read_idx < 32 && offset < buf.len() {
            buf[offset] = self.read_byte(offset + self.read_idx);
            offset += 1;
        }
        self.read_idx += offset; 
        Ok(offset)
    }
}

impl Seek for DirFileEntryData {
    fn seek(&mut self, pos : SeekFrom) -> Result<u64, io::Error> {
        match pos {
            SeekFrom::Start(abs) => {
                self.read_idx = abs as usize;
            },
            SeekFrom::End(back) => {
                let abs = 32 - (back.abs() as usize);
                self.read_idx = abs;
            },
            SeekFrom::Current(off) => {
                if off < 0 {
                    self.read_idx -= off.abs() as usize;
                }
                else {
                    self.read_idx += off.abs() as usize;
                }
            }
        }
        Ok(self.read_idx as u64)
    }
}




#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
struct FileAttributes(u8);

impl FileAttributes {
    const READ_ONLY : u8 = 0x01;
    const HIDDEN : u8 = 0x02;
    const SYSTEM : u8 = 0x04;
    const VOLUME_ID : u8 = 0x08;
    const DIRECTORY : u8 = 0x10;
    const ARCHIVE : u8 = 0x20;

    pub fn file() -> FileAttributes {
        FileAttributes(0)
    }

    pub fn directory() -> FileAttributes {
        FileAttributes(FileAttributes::DIRECTORY)
    }

    pub fn volume_label() -> FileAttributes {
        FileAttributes(FileAttributes::VOLUME_ID)
    }
    
    pub fn and_read_only(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::READ_ONLY)
    }

    pub fn and_hidden(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::HIDDEN)
    }

    pub fn and_system(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::SYSTEM)
    }

    pub fn and_volume_id(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::VOLUME_ID)
    }

    pub fn and_directory(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::DIRECTORY)
    }

    pub fn and_archive(self) -> FileAttributes {
        FileAttributes(self.0 | FileAttributes::ARCHIVE)
    }

    pub fn is_read_only(self) -> bool {
        self.0 & FileAttributes::READ_ONLY != 0
    }
    
    pub fn is_hidden(self) -> bool {
        self.0 & FileAttributes::HIDDEN != 0
    }
    
    pub fn is_system(self) -> bool {
        self.0 & FileAttributes::SYSTEM != 0
    }

    pub fn is_volume_id(self) -> bool {
        self.0 & FileAttributes::VOLUME_ID != 0
    }

    pub fn is_directory(self) -> bool {
        self.0 & FileAttributes::DIRECTORY != 0
    }

    pub fn is_archive(self) -> bool {
        self.0 & FileAttributes::ARCHIVE != 0
    }

    pub fn is_volume_label(self) -> bool {
        !self.is_long_file_name() && !self.is_directory() && self.is_volume_id()
    }

    pub fn is_file(self) -> bool {
        !self.is_directory() && !self.is_volume_id()
    }

    pub fn is_long_file_name(self) -> bool {
        self.is_read_only() && self.is_system() && self.is_hidden() && self.is_volume_id()
    }
}


impl BitAnd<u8> for FileAttributes {
    type Output = FileAttributes;
    fn bitand(self, rhs: u8) -> FileAttributes {
        FileAttributes(self.0 & rhs)
    }
}


impl BitAnd<FileAttributes> for u8 {
    type Output = FileAttributes;
    fn bitand(self, rhs: FileAttributes) -> FileAttributes {
        FileAttributes(rhs.0 & self)
    }
}

impl From<FileAttributes> for u8 {
    fn from(wrapped : FileAttributes) -> u8 {
        wrapped.0
    }
}