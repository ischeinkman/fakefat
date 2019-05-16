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

mod stdimpl;
pub use stdimpl::*;

mod fsinfo;
pub use fsinfo::*;

/*
use fatfs;
pub fn main() {
    let test_faker = FakeFat::new(StdFileSystem{}, "/home/ilan/testfata/".to_owned());
    let test_fs = fatfs::FileSystem::new(test_faker, fatfs::FsOptions::new()).unwrap();
    println!("HELLO!");
    let mut root = test_fs.root_dir();
    println!("HELLO!");
    for itm in root.iter() {
        let ent = itm.unwrap();
        println!("\n\nFound entry: {}\n\n", ent.file_name());
    }
    println!("HELLO!");
}*/
