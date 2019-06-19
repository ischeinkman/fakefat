use crate::datetime::{Date, Time};
use crate::dirent::{FileAttributes, FileDirEntry};

/// Metadata associated with a given file or directory.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileMetadata {
    /// Whether or not this directory entry is a subdirectory; if `false`, this
    /// directory entry represents a file.
    pub is_directory: bool,

    /// Whether or not this child is hidden.
    pub is_hidden: bool,

    /// Whether or not this child cannot be written to.
    pub is_read_only: bool,
    /// The time this child was created.
    pub create_time: Time,
    /// The date this child was created.
    pub create_date: Date,
    /// The date this child was last accessed.
    pub access_date: Date,

    /// The time this child was last modified.
    pub modify_time: Time,
    /// The date this child was last modified.
    pub modify_date: Date,

    /// The size of the file, in bytes. Since the filesystem will use to fake a
    /// FAT32 device, it maxes out at u32::max_value(), or about 4 gb.
    pub size: u32,
}

impl FileMetadata {

    /// Constructs a `FileDirEntry` out of this metadata.
    /// 
    /// By default the returned `FileDirEntry` will have an empty `ShortName`; 
    /// be sure to set it to the correct value before use. 
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

/// Operations that need to be implemented by structs returned by a directory's
/// `entries()` iterator. 
pub trait DirEntryOps {

    /// The string-like type that the directory entry uses for `name()`s. 
    /// 
    /// Generally systems with an `alloc` implementation can use `String`. 
    /// 
    /// In `alloc`-less systems, this can be implemented as `Self` 
    /// with the name value being an inline fixed-length array buffer and the `as_ref()`
    /// function returning that buffer wrapped by a `&str`. 
    type NameType: AsRef<str>;

    /// Returns the name of the item that this entry represents. 
    fn name(&self) -> Self::NameType;

    /// Returns the metadata of the item this entry represents without needing
    /// too look up the item's data itself. 
    fn meta(&self) -> FileMetadata;
}

/// Operations needed for a real backing directory.
pub trait DirectoryOps {
    /// The type of entries that this directory contains.
    type EntryType: DirEntryOps;

    /// The type of struct the directory uses to iterate over its entries.
    type IterType: IntoIterator<Item = Self::EntryType>;

    /// Iterates over this directory's entries.
    fn entries(&self) -> Self::IterType;
}

/// Operations of a real backing file.
pub trait FileOps {
    /// Reads up to `buffer.len()` bytes from the file starting `offset`
    /// bytes from the start of the file, returning the number of bytes read.
    ///
    /// In essence, combines both `Seek::seek` and `Read::read` into a single function.
    fn read_at(&mut self, offset: usize, buffer: &mut [u8]) -> usize;
}

/// Operations that must be implemented by the real "file system" that will be exposed
/// as a FAT32 file system. 
pub trait FileSystemOps {

    /// The directory entry struct that this FileSystem uses. 
    type DirEntryType: DirEntryOps;
    
    /// The directory struct that this FileSystem uses. 
    type DirectoryType: DirectoryOps<EntryType = Self::DirEntryType>;
    
    /// The file struct that this FileSystem uses. 
    type FileType: FileOps;

    /// Attempts to find a file with the given path.
    /// 
    /// Returns `None` if `path` does not represent an already existing 
    /// non-directory file. 
    fn get_file(&mut self, path: &str) -> Option<Self::FileType>;
    /// Attempts to find a directory with the given path.
    /// 
    /// Returns `None` if `path` does not represent an already existing 
    /// non-file directory. 
    fn get_dir(&mut self, path: &str) -> Option<Self::DirectoryType>;


    /// Attempts to find metadata about an item with the given path.
    /// 
    /// Returns `None` if `path` does not represent an already existing 
    /// file or directory. 
    fn get_metadata(&mut self, path: &str) -> Option<FileMetadata>;
}
