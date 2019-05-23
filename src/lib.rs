#![cfg_attr(not(feature = "std"), no_std)]
//#[macro_use]
#[cfg(feature="alloc")]
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
pub use stdimpl::{StdFileSystem};

mod fsinfo;
pub use fsinfo::*;

mod clustermapping;

mod pathbuffer;

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