use crate::bpb::{default_sectors_per_fat, BiosParameterBlock};
use crate::dirent::{FileDirEntry, LfnDirEntry, ENTRY_SIZE};
use crate::fat::{idx_to_cluster, FatEntryValue};
use crate::fsinfo::FsInfoSector;
use crate::longname::{construct_name_entries, lfn_count_for_name};
use crate::shortname::ShortName;
use crate::traits::{DirEntryOps, DirectoryOps, FileMetadata, FileOps, FileSystemOps};

use std::collections::BTreeMap;
use std::vec::Vec;

pub struct FakeFat<T: FileSystemOps> {
    bpb: BiosParameterBlock,
    fsinfo: FsInfoSector,
    fs: T,
    cluster_mapping: BTreeMap<u32, String>,
    path_mapping: BTreeMap<String, Vec<u32>>,
    metadata_cache: BTreeMap<String, FileMetadata>,
    _path_prefix: String,

    read_idx: usize,
}

impl<T: FileSystemOps> FakeFat<T> {
    pub fn new(mut fs: T, path_prefix: String) -> Self {
        let mut cluster_mapping: BTreeMap<u32, String> = BTreeMap::new();
        let mut path_mapping: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        let mut metadata_cache: BTreeMap<String, FileMetadata> = BTreeMap::new();
        metadata_cache.insert(
            path_prefix.clone(),
            FileMetadata {
                is_directory: true,
                ..Default::default()
            },
        );

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
                metadata_cache.insert(path.clone(), meta);
                entry_count += 1 + lfn_count_for_name(&name);
                if meta.is_directory {
                    path_queue.push(format!("{}/", path));
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
        let total_clusters = (bpb.root_dir_first_cluster + cur_cluster + 1).max(0xABCDEF);
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
            metadata_cache,
            _path_prefix : path_prefix,
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
        if idx < BiosParameterBlock::SIZE {
            let retval = self.bpb.read_byte(idx);
            return retval;
        } else if idx < BiosParameterBlock::SIZE + FsInfoSector::SIZE {
            let retval = self.fsinfo.read_byte(idx - BiosParameterBlock::SIZE);
            return retval;
        } else if idx > self.fat_start() && idx < self.fat_end() {
            let cluster = idx_to_cluster(&self.bpb, idx);
            let cur_value = {
                let cur_path = self.cluster_mapping.get(&cluster);
                if let Some(cp) = cur_path {
                    let cur_chain = self.path_mapping.get(cp);
                    let next_link = cur_chain
                        .and_then(|chain| chain.iter().skip_while(|l| **l != cluster).next());
                    next_link.map(|&c| c.into()).unwrap_or(FatEntryValue::Bad)
                } else {
                    FatEntryValue::Free
                }
            };
            let entry_bytes: u32 = cur_value.into();
            let offset = idx % 4;
            let shift = offset * 8;
            let retval = ((entry_bytes & (0xFF << shift)) >> shift) as u8;
            return retval;
        } else {
            let cluster_size = self.bpb.bytes_per_cluster() as usize;
            let data_begin_offset = self.fat_end();
            let cluster = ((idx - data_begin_offset) / cluster_size) as u32;
            let path = match self.cluster_mapping.get(&cluster) {
                Some(p) => p,
                None => {
                    return 0;
                }
            };

            let cluster_chain = self.path_mapping.get(path).unwrap();
            let clusters_previous = cluster_chain.iter().take_while(|c| **c != cluster).count();
            let byte_offset =
                clusters_previous * cluster_size + ((idx - data_begin_offset) % cluster_size);

            let meta = self.metadata_cache.get(path).unwrap();
            if meta.is_directory {
                let dir = self.fs.get_dir(path).unwrap();
                let sys_entries = dir.entries();
                let fat_entries = sys_entries.iter().map(|ent| {
                    let stripped_name: String = ent.name().as_ref().to_owned();
                    (
                        stripped_name.clone(),
                        file_to_direntries(&stripped_name, ent.meta()),
                    )
                });
                let entry_num = byte_offset / ENTRY_SIZE;
                let mut cur_idx = 0;
                for (full_name, (mut file_ent, name_ents)) in fat_entries {
                    if 1 + name_ents.len() + cur_idx <= entry_num {
                        cur_idx += 1 + name_ents.len();
                        continue;
                    }
                    let entry_offset = entry_num - cur_idx;
                    let entry_byte = byte_offset % ENTRY_SIZE;

                    if entry_offset == name_ents.len() {
                        let full_path = if file_ent.attrs.is_directory() {
                            format!("{}{}/", path, full_name)
                        } else {
                            format!("{}{}", path, full_name)
                        };
                        file_ent.first_cluster = *self
                            .path_mapping
                            .get(&full_path)
                            .and_then(|v| v.get(0))
                            .unwrap();
                        return file_ent.read_byte(entry_byte);
                    } else {
                        let name_ent_idx = name_ents.len() - entry_offset - 1;
                        assert!(name_ents[name_ent_idx].attrs.is_long_file_name());
                        assert_eq!(name_ents[name_ent_idx].entry_num & 0x3f, name_ent_idx as u8);
                        return name_ents[name_ents.len() - entry_offset - 1].read_byte(entry_byte);
                    }
                }
                return 0;
            } else {
                let mut fl = self.fs.get_file(path).unwrap();
                let mut buff = [0; 1];
                let _read = fl.read_at(byte_offset, &mut buff);
                buff[0]
            }
        }
    }
}

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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
fn file_to_direntries(name: &str, meta: FileMetadata) -> (FileDirEntry, Vec<LfnDirEntry>) {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let full_idx = hasher.finish();
    let idx = full_idx % 256;
    let mut fileent = meta.to_dirent();
    let short_name = ShortName::convert_str(name, idx as u8);
    fileent.name = short_name;
    (fileent, construct_name_entries(name, fileent))
}
