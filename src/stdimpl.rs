use crate::datetime::{Date, Time};
use crate::traits::{DirEntryOps, DirectoryOps, FileMetadata, FileOps, FileSystemOps};
use std::fs::{self, DirEntry, File, Metadata};
use std::io::{self, Read, Seek};
use std::path::PathBuf;
use std::time::SystemTime;

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
    type IterType = Vec<DirEntry>;
    fn entries(&self) -> Vec<DirEntry> {
        fs::read_dir(&self)
            .map(|iter| iter.map(Result::unwrap).collect())
            .unwrap()
    }
}

/// An implementation of `FileSystemOps` using Rust's `std::fs` module.
pub struct StdFileSystem {}

impl FileSystemOps for StdFileSystem {
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

    fn get_metadata(&mut self, path: &str) -> Option<FileMetadata> {
        match fs::metadata(path) {
            Ok(mt) => Some(get_metadata(mt)),
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
        Date::from_epoch_millis(millis_since_epoch),
        Time::from_epoch_millis(millis_since_epoch),
    )
}
