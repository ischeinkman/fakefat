use crate::bpb::{default_sectors_per_fat, BiosParameterBlock};
use crate::clustermapping::{ClusterMapper, ClusterMapperOps};
use crate::dirent::{EmptyDirEntry, FileDirEntry, LfnDirEntry, ENTRY_SIZE};
use crate::fat::{idx_to_cluster, FatEntryValue};
use crate::fsinfo::FsInfoSector;
use crate::longname::{construct_name_entries, lfn_count_for_name};
use crate::pathbuffer::PathBuff;
use crate::shortname::ShortName;
use crate::traits::{DirEntryOps, DirectoryOps, FileMetadata, FileOps, FileSystemOps};
use crate::ReadByte;

use core::num::Wrapping;

/// Wraps any filesystem and exposes it as if it was a normal FAT32
/// device that can be either read byte-by-byte or via the normal `Read` and `Seek`
/// traits without actually touching the backing filesystem itself.
pub struct FakeFat<T: FileSystemOps> {
    bpb: BiosParameterBlock,
    fsinfo: FsInfoSector,
    fs: T,
    mapper: ClusterMapper,
    read_idx: usize,
}

use core::ops::Index;

fn traverse<T: FileSystemOps>(
    mapper: &mut ClusterMapper,
    cur: &PathBuff,
    fs: &mut T,
    bytes_per_cluster: usize,
) -> u32 {
    let entry_count: usize = fs
        .get_dir(cur.to_str())
        .unwrap()
        .entries()
        .into_iter()
        .map(|ent| 1 + lfn_count_for_name(ent.name().as_ref()))
        .sum();
    let needed_bytes = entry_count.max(1) * ENTRY_SIZE;
    let needed_clusters = needed_bytes / bytes_per_cluster
        + if needed_bytes % bytes_per_cluster == 0 {
            0
        } else {
            1
        };
    let mut cur_cluster = 0;
    let mut clusters = 0;
    while clusters < needed_clusters {
        while mapper.is_allocated(cur_cluster) {
            cur_cluster += 1;
        }
        mapper.add_cluster_to_path(cur.to_str(), cur_cluster);
        clusters += 1;
    }

    let mut max_cluster = cur_cluster;

    let subdirs = fs
        .get_dir(cur.to_str())
        .unwrap()
        .entries()
        .into_iter()
        .filter(|ent| ent.meta().is_directory);
    let subfiles = fs
        .get_dir(cur.to_str())
        .unwrap()
        .entries()
        .into_iter()
        .filter(|ent| !ent.meta().is_directory);
    for ent in subfiles {
        let nh = ent.name();
        let path = {
            let mut r = PathBuff::default();
            r.add_subdir(cur.to_str());
            r.add_file(nh.as_ref());
            r
        };
        let meta = ent.meta();
        let needed_clusters = meta.size as usize / bytes_per_cluster
            + if meta.size as usize % bytes_per_cluster == 0 {
                0
            } else {
                1
            };
        let mut clusters = 0;
        while clusters < needed_clusters {
            let mut my_offset = cur_cluster + 12;
            while mapper.is_allocated(my_offset) {
                my_offset += 1;
            }
            clusters += 1;
            mapper.add_cluster_to_path(path.to_str(), my_offset);
            max_cluster = max_cluster.max(my_offset);
        }
    }

    for dir in subdirs {
        let path_comp = dir.name();
        let path = {
            let mut r = PathBuff::default();
            r.add_subdir(cur.to_str());
            r.add_subdir(path_comp.as_ref());
            r
        };
        max_cluster = max_cluster.max(traverse(mapper, &path, fs, bytes_per_cluster));
    }
    max_cluster
}

impl<T: FileSystemOps> FakeFat<T> {
    /// Constructs a new Fake FAT32 device wrapping the given filesystem.
    /// `path_prefix` represents where in the real filesystem should map to the
    /// FAT32 device's root directory; for a direct one-to-one mapping, use `"/"`.
    pub fn new(mut fs: T, path_prefix: &str) -> Self {
        let path_prefix = {
            let mut r = PathBuff::default();
            r.add_subdir(path_prefix);
            r
        };
        let mut bpb = BiosParameterBlock::default();
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 8;
        let mut mapper = ClusterMapper::new();

        let max_cluster = traverse(
            &mut mapper,
            &path_prefix,
            &mut fs,
            bpb.bytes_per_cluster() as usize,
        );
        let total_clusters = (bpb.root_dir_first_cluster + max_cluster + 1).max(0xAB_CDEF);
        let total_sectors = u32::from(bpb.sectors_per_cluster) * total_clusters;
        bpb.total_sectors_32 = total_sectors;
        let spf = default_sectors_per_fat(&bpb);
        bpb.sectors_per_fat_32 = spf;
        Self {
            bpb,
            fsinfo: FsInfoSector::default(),
            fs,
            mapper,
            read_idx: 0,
        }
    }

    /// Reads a single byte out of the FAT32 device, exactly `idx` bytes from the
    /// head of the device.
    pub fn read_byte(&mut self, idx: usize) -> u8 {
        match FakerAddress::from_raw_idx(idx, &self.bpb) {
            FakerAddress::Bpb(bpb_idx) => self.bpb.read_byte(bpb_idx),
            FakerAddress::FsInfo(fs_idx) => self.fsinfo.read_byte(fs_idx),
            FakerAddress::Fat { cluster, byte } => {
                let cur_value = {
                    // Is it associated to a path?
                    let cur_path = self.mapper.get_path_for_cluster(cluster);
                    if let Some(cp) = cur_path {
                        // If so, get the path's chain and find the next link, if there is one.
                        // Otherwise return the Chain End marker value.
                        let cur_chain = self.mapper.get_chain_for_path(cp);
                        let next_link = cur_chain.into_iter().skip_while(|&l| l != cluster).next();
                        next_link.map(|c| c.into()).unwrap_or(FatEntryValue::End)
                    } else {
                        // If not, the cluster is free.
                        FatEntryValue::Free
                    }
                };

                // Get the actual byte we need from the 4 byte entry.
                let entry_bytes: u32 = cur_value.into();
                let shift = byte * 8;
                ((entry_bytes & (0xFF << shift)) >> shift) as u8
            }
            FakerAddress::RawData { cluster, offset } => {
                match FakerDataAddress::resolve_raw_data(
                    cluster,
                    offset,
                    &self.bpb,
                    &self.mapper,
                    &mut self.fs,
                ) {
                    None => 0,
                    Some(FakerDataAddress::File { mut file, offset }) => {
                        file.read_byte(offset).unwrap_or(0)
                    }
                    Some(FakerDataAddress::Directory {
                        directory,
                        entry,
                        offset,
                    }) => {
                        // Directories are composed of 1 shortname-styled entry per subitem
                        // Plus an arbitrary number of LFN entries.
                        // We "build" all of them here to figure out which of all the entries we are reading from.
                        // Note that thanks to how Rust iterators work, this is done lazily even without an std!
                        let sys_entries = directory.entries();
                        let fat_entries = sys_entries.into_iter().map(|ent| {
                            let dirents = file_to_direntries(ent.name().as_ref(), ent.meta());
                            (ent, dirents)
                        });
                        let mut cur_idx = 0;

                        // Go through each subitems combined entries
                        for (ent, (mut file_ent, name_ents)) in fat_entries {
                            let full_name = ent.name();

                            // Skip until we reach the correct LFN-shortname combo
                            if 1 + name_ents.len() + cur_idx <= entry {
                                cur_idx += 1 + name_ents.len();
                                continue;
                            }

                            // Calculate the entry in this lfn-shortname list we need,
                            // and the byte within that entry.
                            let entry_offset = entry - cur_idx;

                            if entry_offset == name_ents.len() {
                                let path = self.mapper.get_path_for_cluster(cluster).unwrap();
                                // The shortname entry is considered to be 1 after the final lfn entry.
                                let mut full_path = PathBuff::default();
                                full_path.add_subdir(path);
                                if file_ent.attrs.is_directory() {
                                    full_path.add_subdir(full_name.as_ref());
                                } else {
                                    full_path.add_file(full_name.as_ref());
                                }

                                file_ent.first_cluster = self
                                    .mapper
                                    .get_chain_for_path(full_path.to_str())
                                    .into_iter()
                                    .next()
                                    .map(|c| c + 2 as u32) // Add 2 since FAT32 has 2 reserved clusters? I think?
                                    .unwrap();
                                return file_ent.read_byte(offset);
                            } else {
                                // The LFN entries are actually in reverse order to the string itself.
                                let name_ent_idx = name_ents.len() - entry_offset - 1;
                                return name_ents[name_ent_idx].read_byte(offset);
                            }
                        }

                        // If we couldn't find the path for the entry, the entry is empty.
                        let entry_byte = offset;
                        EmptyDirEntry::default().read_byte(entry_byte)
                    }
                }
            }
        }
    }
}

enum FakerAddress {
    Bpb(usize),
    FsInfo(usize),
    Fat { cluster: u32, byte: u8 },
    RawData { cluster: u32, offset: usize },
}

impl FakerAddress {
    pub fn from_raw_idx(idx: usize, bpb: &BiosParameterBlock) -> Self {
        // The first 1024 bytes are the BPB and the FSInfo
        if idx < BiosParameterBlock::SIZE {
            FakerAddress::Bpb(idx)
        } else if idx < BiosParameterBlock::SIZE + FsInfoSector::SIZE {
            FakerAddress::FsInfo(idx - BiosParameterBlock::SIZE)
        }
        // Next comes the table of allocations and chains, aka the File Allocation Table.
        else if idx >= bpb.fat_start() && idx < bpb.fat_end() {
            // Gets the cluster that we need to get the entry of.
            let cluster = idx_to_cluster(bpb, idx);
            let byte = (idx % 4) as u8;
            FakerAddress::Fat { cluster, byte }
        } else {
            let cluster_size = bpb.bytes_per_cluster() as usize;

            // Our data starts where the FAT ends.
            let data_begin_offset = bpb.fat_end();

            // The cluster and path we are reading from.
            let cluster = ((idx - data_begin_offset) / cluster_size) as u32;
            let offset = (idx - data_begin_offset) % cluster_size;
            FakerAddress::RawData { cluster, offset }
        }
    }
}

enum FakerDataAddress<F: FileOps, D: DirectoryOps> {
    File {
        file: F,
        offset: usize,
    },
    Directory {
        directory: D,
        entry: usize,
        offset: usize,
    },
}

impl<D: DirectoryOps, F: FileOps> FakerDataAddress<F, D> {
    pub fn resolve_raw_data<
        MapType: ClusterMapperOps,
        FS: FileSystemOps<DirectoryType = D, FileType = F>,
    >(
        cluster: u32,
        offset: usize,
        bpb: &BiosParameterBlock,
        mapper: &MapType,
        fs: &mut FS,
    ) -> Option<Self> {
        let path = mapper.get_path_for_cluster(cluster)?;
        // We need to go from offset in the fake device to offset in the real file or directory.
        // To do so, we first convert from device offset to offset in this cluster chain.
        let cluster_chain = mapper.get_chain_for_path(path);
        let clusters_previous = cluster_chain
            .into_iter()
            .take_while(|&c| c != cluster)
            .count();
        let byte_offset = clusters_previous * (bpb.bytes_per_cluster() as usize) + offset;
        let meta = fs.get_metadata(path)?;
        if meta.is_directory {
            Some(FakerDataAddress::Directory {
                directory: fs.get_dir(path)?,
                entry: byte_offset / ENTRY_SIZE,
                offset: (byte_offset % ENTRY_SIZE),
            })
        } else {
            Some(FakerDataAddress::File {
                file: fs.get_file(path)?,
                offset: byte_offset,
            })
        }
    }
}

pub use stdio::*;
#[cfg(not(feature = "std"))]
mod stdio {}

#[cfg(feature = "std")]
mod stdio {
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

fn file_to_direntries(name: &str, meta: FileMetadata) -> (FileDirEntry, LfnChain) {
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
    let mut allocation = LfnChain::default();
    construct_name_entries(name, fileent, &mut allocation);
    allocation.len = lfn_length;
    (fileent, allocation)
}

#[derive(Copy, Clone, Default)]
struct LfnChain {
    len: usize,
    allocation: [LfnDirEntry; 32],
}

impl LfnChain {
    fn len(&self) -> usize {
        self.len
    }
}

impl Index<usize> for LfnChain {
    type Output = LfnDirEntry;

    fn index(&self, idx: usize) -> &LfnDirEntry {
        &self.allocation[idx]
    }
}

impl AsMut<[LfnDirEntry]> for LfnChain {
    fn as_mut(&mut self) -> &mut [LfnDirEntry] {
        &mut self.allocation
    }
}
