#![warn(missing_docs)]
#![allow(clippy::identity_conversion)]
#![allow(clippy::or_fun_call)]
#![cfg_attr(not(feature = "std"), no_std)]

//! This crate allows any filesystem-like entity to be exposed as a FAT32-formated
//! disk image on the fly. 

//#[macro_use]
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

mod shortname;
pub use shortname::*;

mod dirent;
pub use dirent::*;

mod bpb;
pub use bpb::*;

mod datetime;
pub use datetime::*;

mod traits;
pub use traits::*;

mod longname;
pub use longname::*;

mod fat;
pub use fat::*;

mod faker;
pub use faker::*;

#[cfg(feature = "std")]
mod stdimpl;
#[cfg(feature = "std")]
pub use stdimpl::StdFileSystem;

mod fsinfo;
pub use fsinfo::*;

mod clustermapping;

mod pathbuffer;

mod changeset;

/// Allows to use the structs that represent the sections of the fake filesystem
/// as a byte slice without having to actually generate the byte slice, since 
/// much of the time the array the section represents is mostly empty space. 
pub trait ReadByte {

    /// The number of bytes this struct represents if it was backed by a literal
    /// byte array.
    const SIZE: usize;

    /// Gets a byte out of the "array" at the specified index. 
    fn read_byte(&self, idx: usize) -> u8;

    /// Gets multiple bytes out of the "array," starting at the specified index. 
    /// Returns the number of bytes read, which in most cases will be `(Self::SIZE - idx).min(idx + buffer.len())`.
    fn read_at(&self, idx : usize, buffer : &mut [u8]) -> usize {
        let end_idx = (idx + buffer.len()).min(Self::SIZE);
        for cur_idx in idx .. end_idx {
            let buff_idx = cur_idx - idx; 
            buffer[buff_idx] = self.read_byte(cur_idx);
        }
        end_idx - idx
    }
}

/*
#[cfg(feature="std")]
use fatfs;
#[cfg(feature="std")]
pub fn main() {
    simple_logger::init_with_level(log::Level::max())
        .unwrap();
    let test_faker = FakeFat::new(StdFileSystem{}, "/home/ilan/testfata/");
    let test_fs = fatfs::FileSystem::new(test_faker, fatfs::FsOptions::new()).unwrap();
    println!("HELLO!");
    let root = test_fs.root_dir();
    println!("HELLO!");
    utils::transverse("/".to_owned(), root);
    println!("HELLO!");
}
#[cfg(feature="std")]
mod utils {
    pub fn transverse<'a, T : fatfs::ReadWriteSeek>(cur_path : String, start : fatfs::Dir<'a, T>) {
        let mut queue = vec![(cur_path, start)];
        while let Some((path, dir)) = queue.pop() {
            println!("\n\n ---  Traversing {}  --- \n\n", path);
            for entres in dir.iter() {
                let itm = entres.unwrap();
                println!("\n\nEntry:    {}/{}\n\n", path, itm.file_name());
                if itm.is_dir() {
                    queue.push(((format!("{}/{}", path, itm.file_name())), itm.to_dir()));
                }
            }
        }
    }
}



#[cfg(not(feature="std"))]
pub fn main() {}
*/
