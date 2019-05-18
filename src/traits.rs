use crate::datetime::{Date, Time};
use crate::dirent::{FileAttributes, FileDirEntry};

#[derive(Copy, Clone, Debug, Default)]
pub struct FileMetadata {
    pub is_directory: bool,
    pub is_hidden: bool,
    pub is_read_only: bool,
    pub create_time: Time,
    pub create_date: Date,
    pub access_date: Date,
    pub modify_time: Time,
    pub modify_date: Date,
    pub size: u32,
}

impl FileMetadata {
    pub fn to_dirent(&self) -> FileDirEntry {
        let mut retval = FileDirEntry::default();
        retval.create_time = self.create_time;
        retval.create_date = self.create_date;
        retval.modify_time = self.modify_time;
        retval.modify_date = self.modify_date;
        retval.access_date = self.access_date;
        retval.size = self.size;
        let attrs = if self.is_directory {
            FileAttributes::directory()
        } else {
            FileAttributes::file()
        };
        let attrs = if self.is_hidden {
            attrs.and_hidden()
        } else {
            attrs
        };
        let attrs = if self.is_read_only {
            attrs.and_read_only()
        } else {
            attrs
        };
        retval.attrs = attrs;
        retval
    }
}

pub trait DirEntryOps {
    type NameType: AsRef<str>;
    fn name(&self) -> Self::NameType;
    fn meta(&self) -> FileMetadata;
}

pub trait DirectoryOps {
    type EntryType: DirEntryOps;
    type IterType : IntoIterator<Item=Self::EntryType>;
    fn entries(&self) -> Self::IterType;
}

pub trait FileOps {
    fn read_at(&mut self, offset: usize, buffer: &mut [u8]) -> usize;
}

pub trait FileSystemOps {
    type DirEntryType: DirEntryOps;
    type DirectoryType: DirectoryOps<EntryType = Self::DirEntryType>;
    type FileType: FileOps;

    fn get_file(&mut self, path: &str) -> Option<Self::FileType>;
    fn get_dir(&mut self, path: &str) -> Option<Self::DirectoryType>;
    fn get_metadata(&mut self, path:&str) -> Option<FileMetadata>;
}
