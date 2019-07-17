use crate::datetime::{Date, Time};
use crate::shortname::ShortName;
use crate::ReadByte;
use core::ops::BitAnd;

/// All directory entries, whether a child entry, Long File Name chain link,
/// or just empty, span exactly 32 bytes.
pub(crate) const ENTRY_SIZE: usize = 32;

/// An entry in a directory that represents a child item, as opposed to a Long
/// File Name.
#[derive(Clone, Debug, Default, Copy)]
pub struct FileDirEntry {
    pub(crate) name: ShortName,
    pub(crate) attrs: FileAttributes,
    pub(crate) create_time: Time,
    pub(crate) create_date: Date,
    pub(crate) access_date: Date,
    pub(crate) first_cluster: u32,
    pub(crate) modify_time: Time,
    pub(crate) modify_date: Date,
    pub(crate) size: u32,
}

impl ReadByte for FileDirEntry {
    const SIZE: usize = ENTRY_SIZE;
    fn read_byte(&self, idx: usize) -> u8 {
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

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Default, Hash)]
pub(crate) struct FileAttributes(u8);

impl FileAttributes {
    const READ_ONLY: u8 = 0x01;
    const HIDDEN: u8 = 0x02;
    const SYSTEM: u8 = 0x04;
    const VOLUME_ID: u8 = 0x08;
    const DIRECTORY: u8 = 0x10;
    const ARCHIVE: u8 = 0x20;

    pub fn file() -> FileAttributes {
        FileAttributes(0)
    }

    pub fn directory() -> FileAttributes {
        FileAttributes(FileAttributes::DIRECTORY)
    }

    pub fn volume_label() -> FileAttributes {
        FileAttributes(FileAttributes::VOLUME_ID)
    }

    pub fn lfn() -> FileAttributes {
        FileAttributes::volume_label()
            .and_read_only()
            .and_hidden()
            .and_system()
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
    fn from(wrapped: FileAttributes) -> u8 {
        wrapped.0
    }
}

/// An entry in a Long File Name chain.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct LfnDirEntry {
    pub(crate) entry_num: u8,
    pub(crate) attrs: FileAttributes,
    pub(crate) checksum: u8,
    pub(crate) name_part: [u8; 13],
}

impl Default for LfnDirEntry {
    fn default() -> LfnDirEntry {
        LfnDirEntry {
            entry_num: 0,
            attrs: FileAttributes::lfn(),
            checksum: 0,
            name_part: [0; 13],
        }
    }
}

impl ReadByte for LfnDirEntry {
    const SIZE: usize = ENTRY_SIZE;
    fn read_byte(&self, idx: usize) -> u8 {
        match idx {
            0 => self.entry_num,
            1 => self.name_part[0],
            3 => self.name_part[1],
            5 => self.name_part[2],
            7 => self.name_part[3],
            9 => self.name_part[4],
            11 => self.attrs.0,
            12 => 0,
            13 => self.checksum,
            14 => self.name_part[5],
            16 => self.name_part[6],
            18 => self.name_part[7],
            20 => self.name_part[8],
            22 => self.name_part[9],
            24 => self.name_part[10],
            28 => self.name_part[11],
            30 => self.name_part[12],
            _ => 0,
        }
    }
}

/// An entry allocated in a given directory's cluster chain that has not yet
/// been filled with either a child entry or part of a Long File Name chain.
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct EmptyDirEntry {}
impl ReadByte for EmptyDirEntry {
    const SIZE: usize = ENTRY_SIZE;
    fn read_byte(&self, idx: usize) -> u8 {
        match idx {
            0 => 0x00,
            11 => 0x40,
            _ => 0,
        }
    }
}

/// An entry in a Fat32 directory. 
/// 
/// A FAT32 directory can be thought of as a number of 32-byte "slots" to represent
/// a number of children items. Each child is composed of zero or more "file name"-style
/// entries, followed by the actual "child" entry containing a variety of metadata
/// and a pointer to the beginning of the content body. A slot's current "kind" is determined
/// by status flags in the slot's index-11 byte: 
/// 
/// * If the most-significant bit is set to 1, the slot is empty. 
/// * If the entirety of the least significant nibble is set to 0xF, the slot is part of a file name chain. 
/// * Otherwise, it is a standard child entry. 
#[derive(Copy, Clone, Debug)]
pub enum Fat32DirectoryEntry {

    /// A directory entry containing metadata for a child item. 
    File(FileDirEntry),

    /// A directory entry containing part of a file name chain. 
    LongFileName(LfnDirEntry), 

    /// A directory entry containing no data. 
    Empty(EmptyDirEntry), 
}

impl Fat32DirectoryEntry {

    /// Constructs a new empty entry. 
    pub const fn empty() -> Self {
        Fat32DirectoryEntry::Empty(EmptyDirEntry{})
    }
}

impl Default for Fat32DirectoryEntry {
    fn default() -> Self {
        Fat32DirectoryEntry::empty()
    }
}

impl ReadByte for Fat32DirectoryEntry {
    const SIZE : usize = ENTRY_SIZE;
    fn read_byte(&self, idx: usize) -> u8 {
        match self {
            Fat32DirectoryEntry::File(f) => f.read_byte(idx), 
            Fat32DirectoryEntry::LongFileName(f) => f.read_byte(idx), 
            Fat32DirectoryEntry::Empty(f) => f.read_byte(idx), 
        }
    }
}

impl From<FileDirEntry> for Fat32DirectoryEntry {
    fn from(inner : FileDirEntry) -> Self {
        Fat32DirectoryEntry::File(inner)
    }
}
impl From<LfnDirEntry> for Fat32DirectoryEntry {
    fn from(inner : LfnDirEntry) -> Self {
        Fat32DirectoryEntry::LongFileName(inner)
    }
}
impl From<EmptyDirEntry> for Fat32DirectoryEntry {
    fn from(inner : EmptyDirEntry) -> Self {
        Fat32DirectoryEntry::Empty(inner)
    }
}