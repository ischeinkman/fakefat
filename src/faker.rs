use crate::bpb::{default_sectors_per_fat, BiosParameterBlock};
use crate::dirent::{FileDirEntry, LfnDirEntry, EmptyDirEntry, ENTRY_SIZE};
use crate::fat::{idx_to_cluster, FatEntryValue};
use crate::fsinfo::FsInfoSector;
use crate::longname::{construct_name_entries, lfn_count_for_name};
use crate::shortname::ShortName;
use crate::traits::{DirEntryOps, DirectoryOps, FileMetadata, FileOps, FileSystemOps};

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use core::num::Wrapping;

pub struct FakeFat<T: FileSystemOps> {
    bpb: BiosParameterBlock,
    fsinfo: FsInfoSector,
    fs: T,
    cluster_mapping: BTreeMap<u32, String>,
    path_mapping: BTreeMap<String, Vec<u32>>,
    read_idx: usize,
}

impl<T: FileSystemOps> FakeFat<T> {
    pub fn new(mut fs: T, path_prefix: String) -> Self {
        let mut cluster_mapping: BTreeMap<u32, String> = BTreeMap::new();
        let mut path_mapping: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        let mut bpb = BiosParameterBlock::default();
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 1;

        let mut cur_cluster = 0;
        let mut path_queue = vec![path_prefix.clone()];
        while let Some(cur) = path_queue.pop() {
            let mut entry_count = 0;
            let subentries = fs.get_dir(&cur).unwrap().entries();
            for ent in subentries {
                let name = ent.name().as_ref().to_owned();
                let path = format!("{}{}", cur, name);
                let meta = ent.meta();
                entry_count += 1 + lfn_count_for_name(&name);
                if meta.is_directory {
                    let true_path = format!("{}/", path);
                    path_queue.push(true_path);
                } else {
                    let needed_clusters = meta.size / bpb.bytes_per_cluster()
                        + if meta.size % bpb.bytes_per_cluster() == 0 {
                            0
                        } else {
                            1
                        };
                    let mut clusters = Vec::new();
                    while (clusters.len() as u32) < needed_clusters {
                        let mut my_offset = cur_cluster + 12;
                        while cluster_mapping.get(&(my_offset)).is_some() {
                            my_offset += 1;
                        }
                        clusters.push(my_offset);
                        cluster_mapping.insert(my_offset, path.clone());
                    }
                    path_mapping.insert(path.clone(), clusters);
                }
            }

            let needed_bytes = entry_count.max(1) * ENTRY_SIZE;
            let needed_clusters = needed_bytes / (bpb.bytes_per_cluster() as usize)
                + if needed_bytes % (bpb.bytes_per_cluster() as usize) == 0 {
                    0
                } else {
                    1
                };
            let mut clusters = Vec::new();
            while clusters.len() < needed_clusters {
                while cluster_mapping.get(&cur_cluster).is_some() {
                    cur_cluster += 1;
                }
                clusters.push(cur_cluster);
                cluster_mapping.insert(cur_cluster, cur.clone());
            }
            path_mapping.insert(cur.clone(), clusters);
        }
        let total_clusters = (bpb.root_dir_first_cluster + cur_cluster + 1).max(0xAB_CDEF);
        let total_sectors = bpb.sectors_per_cluster as u32 * total_clusters;
        bpb.total_sectors_32 = total_sectors;
        let spf = default_sectors_per_fat(&bpb);
        bpb.sectors_per_fat_32 = spf;
        let retval = Self {
            bpb,
            fsinfo: FsInfoSector::default(),
            fs,
            cluster_mapping,
            path_mapping,
            read_idx: 0,
        };
        retval
    }

    fn fat_start(&self) -> usize {
        self.bpb.reserved_sectors as usize * self.bpb.bytes_per_sector as usize
    }
    fn fat_end(&self) -> usize {
        self.fat_start()
            + (self.bpb.fats as usize)
                * (self.bpb.sectors_per_fat_32 as usize)
                * (self.bpb.bytes_per_sector as usize)
    }
    pub fn read_byte(&mut self, idx: usize) -> u8 {

        // The first 1024 bytes are the BPB and the FSInfo
        if idx < BiosParameterBlock::SIZE {
            let retval = self.bpb.read_byte(idx);
            retval
        } else if idx < BiosParameterBlock::SIZE + FsInfoSector::SIZE {
            let retval = self.fsinfo.read_byte(idx - BiosParameterBlock::SIZE);
            retval
        } 
        
        // Next comes the table of allocations and chains, aka the File Allocation Table.
        else if idx > self.fat_start() && idx < self.fat_end() {

            // Gets the cluster that we need to get the entry of. 
            let cluster = idx_to_cluster(&self.bpb, idx);
            let cur_value = {

                // Is it associated to a path?
                let cur_path = self.cluster_mapping.get(&cluster);
                if let Some(cp) = cur_path {
                    // If so, get the path's chain and find the next link, if there is one. 
                    // Otherwise return the Chain End marker value. 
                    let cur_chain = self.path_mapping.get(cp);
                    let next_link = cur_chain
                        .and_then(|chain| chain.iter().skip_while(|l| **l != cluster).next());
                    next_link.map(|&c| c.into()).unwrap_or(FatEntryValue::End)
                } else {

                    // If not, the cluster is free. 
                    FatEntryValue::Free
                }
            };

            // Get the actual byte we need from the 4 byte entry. 
            let entry_bytes: u32 = cur_value.into();
            let offset = idx % 4;
            let shift = offset * 8;
            let retval = ((entry_bytes & (0xFF << shift)) >> shift) as u8;
            retval
        } 
        
        // Finally comes the raw data itself.
        else {
            let cluster_size = self.bpb.bytes_per_cluster() as usize;

            // Our data starts where the FAT ends. 
            let data_begin_offset = self.fat_end();

            // The cluster and path we are reading from. 
            let cluster = ((idx - data_begin_offset) / cluster_size) as u32;
            let path = match self.cluster_mapping.get(&cluster) {
                Some(p) => p,
                None => {
                    return 0;
                }
            };

            // We need to go from offset in the fake device to offset in the real file or directory. 
            // To do so, we first convert from device offset to offset in this cluster chain. 

            let cluster_chain = self.path_mapping.get(path).unwrap();
            let clusters_previous = cluster_chain.iter().take_while(|c| **c != cluster).count();
            let byte_offset =
                clusters_previous * cluster_size + ((idx - data_begin_offset) % cluster_size);

            // Next, we actually iterate through the data based on what the data is. 
            let meta = self.fs.get_metadata(path).unwrap();
            if meta.is_directory {

                // Directories are composed of 1 shortname-styled entry per subitem 
                // Plus an arbitrary number of LFN entries. 
                // We "build" all of them here to figure out which of all the entries we are reading from. 
                // Note that thanks to how Rust iterators work, this is done lazily even without an std!
                let dir = self.fs.get_dir(path).unwrap();
                let sys_entries = dir.entries();
                let fat_entries = sys_entries.into_iter().map(|ent| {
                    let stripped_name: String = ent.name().as_ref().to_owned();
                    (
                        stripped_name.clone(),
                        file_to_direntries(&stripped_name, ent.meta()),
                    )
                });
                let entry_num = (byte_offset)/ENTRY_SIZE;
                let mut cur_idx = 0;
                eprintln!("Reading Cluster {}, offset {} => directory {}, entry {}.", cluster, byte_offset, path, entry_num);

                // Go through each subitems combined entries
                for (full_name, (mut file_ent, name_ents)) in fat_entries {

                    // Skip until we reach the correct LFN-shortname combo
                    if 1 + name_ents.len() + cur_idx <= entry_num {
                        cur_idx += 1 + name_ents.len();
                        continue;
                    }

                    // Calculate the entry in this lfn-shortname list we need, 
                    // and the byte within that entry.
                    let entry_offset = entry_num - cur_idx;
                    let entry_byte = byte_offset % ENTRY_SIZE;

                    // Just some debug checks.
                    if name_ents.len() == 0 {
                        let tshort = ShortName::from_str(&full_name);
                        debug_assert!(tshort.is_some());
                        debug_assert_eq!(tshort.unwrap(), file_ent.name);
                    }
                    else {
                        debug_assert!(ShortName::from_str(&full_name).is_none());
                    }

                    if entry_offset == name_ents.len() {
                        // The shortname entry is considered to be 1 after the final lfn entry. 
                        let full_path = if file_ent.attrs.is_directory() {
                            format!("{}{}/", path, full_name)
                        } else {
                            format!("{}{}", path, full_name)
                        };
                        file_ent.first_cluster = self
                            .path_mapping
                            .get(&full_path)
                            .and_then(|v| v.get(0))
                            .map(|c| c + 2 as u32) // Add 2 since FAT32 has 2 reserved clusters? I think?
                            .unwrap();
                        eprintln!("    Resolved to shortname entry {}.", file_ent.name.to_str());
                        return file_ent.read_byte(entry_byte);
                    } else {
                        // The LFN entries are actually in reverse order to the string itself. 
                        let name_ent_idx = name_ents.len() - entry_offset - 1;
                        debug_assert!(name_ents[name_ent_idx].attrs.is_long_file_name());
                        debug_assert!(name_ents[name_ent_idx].checksum == file_ent.name.lfn_checksum());
                        eprintln!("    Resolved to lfn entry {} (order {:x})", name_ent_idx, name_ents[name_ent_idx].entry_num);
                        return name_ents[name_ent_idx].read_byte(entry_byte);
                    }
                }

                // If we couldn't find the path for the entry, the entry is empty. 
                let entry_byte = byte_offset % ENTRY_SIZE;
                eprintln!("    Resolved to nothing.");
                EmptyDirEntry::default().read_byte(entry_byte)

            } else {
                // Read the file at the correct byte. 
                let mut fl = self.fs.get_file(path).unwrap();
                let mut buff = [0; 1];
                let _read = fl.read_at(byte_offset, &mut buff);
                buff[0]
            }
        }
    }
}
pub use stdio::*;
#[cfg(not(feature = "std"))]
pub mod stdio {}

#[cfg(feature = "std")]
pub mod stdio {
    use super::*;
    use std::io::{self, Read, Seek, SeekFrom, Write};

    impl<T: FileSystemOps> Read for FakeFat<T> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut cur_idx = 0;
            while cur_idx < buf.len() {
                buf[cur_idx] = self.read_byte(cur_idx + self.read_idx);
                cur_idx += 1;
            }
            self.read_idx += cur_idx;
            Ok(cur_idx)
        }
    }
    impl<T: FileSystemOps> Seek for FakeFat<T> {
        fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
            match pos {
                SeekFrom::Start(abs) => {
                    self.read_idx = abs as usize;
                }
                SeekFrom::End(_back) => {
                    return Err(io::Error::from(io::ErrorKind::InvalidInput));
                }
                SeekFrom::Current(off) => {
                    if off < 0 {
                        self.read_idx -= off.abs() as usize;
                    } else {
                        self.read_idx += off.abs() as usize;
                    }
                }
            }
            Ok(self.read_idx as u64)
        }
    }
    impl<T: FileSystemOps> Write for FakeFat<T> {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
    }

}

fn file_to_direntries(name: &str, meta: FileMetadata) -> (FileDirEntry, Vec<LfnDirEntry>) {
    //TODO: check for duplications.
    let mut fileent = meta.to_dirent();
    let mut idx = Wrapping(0);
    for (_charnum, bt) in name.as_bytes().iter().enumerate() {
        let offset = bt.wrapping_sub(b'A');
        let bottom_bits = offset & 0xF;
        idx <<= 1;
        idx ^= Wrapping(bottom_bits);
    }
    let short_name = ShortName::convert_str(name, idx.0);
    fileent.name = short_name;
    let lfn_length = lfn_count_for_name(name);
    let mut allocation = Vec::with_capacity(lfn_length);
    allocation.resize(lfn_length, Default::default());
    construct_name_entries(name, fileent, &mut allocation);
    (fileent, allocation)
}
