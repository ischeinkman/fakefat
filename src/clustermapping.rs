//! Since this crate should both support less capable environments while also
//! taking advantage of extra features if they are available, the Cluster Mapper
//! is abstracted as a trait with multiple implementations that are toggled
//! based on the used feature flags. Currently, this leads to 2 different
//! `ClusterMapperOps` implementations:
//!
//! *  In environments without an allocator, each Path -> ClusterChain mapping
//! is represented by a fixed-size `FileEntry` struct; the Cluster Mapper is backed
//! by a fixed-size array of entries, with both cluster and path lookups done via
//! linear search.
//!
//! *  In environments with an allocator, the Cluster Mapper is backed by a pair of
//! `HashMaps`: a `HashMap<String, Vec<u32>>` for quick cluster chain lookup, and a
//! `HashMap<u32, String>` for quick path lookup.
//!

pub trait ClusterMapperOps {
    type ChainIterator: IntoIterator<Item = u32>;

    /// Constructs a Cluster Mapper without any mappings.
    fn new() -> Self;

    /// Gets the path to which this cluster is allocated for, or `None` if the
    /// cluster has not yet been allocated.
    fn get_path_for_cluster(&self, cluster: u32) -> Option<&str>;

    /// Returns a view over the clusters allocated for a particular path.
    ///
    /// If the path has not yet been allocated, the iterator will be empty.   
    fn get_chain_for_path(&self, path: &str) -> Self::ChainIterator;

    /// Appends a cluster to the end of the cluster chain associated with the given
    /// `path`; if there is no chain associated with `path` yet, it is created with
    /// `cluster` as its single link.
    fn add_cluster_to_path(&mut self, path: &str, cluster: u32);

    /// Returns whether a given `cluster` is currently in any allocated cluster chain.
    fn is_allocated(&self, cluster: u32) -> bool;

    /// Attempts to find the chain containing the given cluster, returning `None` otherwise. 
    fn get_chain_with_cluster(&self, cluster: u32) -> Option<Self::ChainIterator> {
        self.get_path_for_cluster(cluster)
            .map(|p| self.get_chain_for_path(p))
    }

    /// Gets the first cluster in the chain associated with a given path, or 
    /// `None` if the path has not yet been associated with a chain. 
    fn get_chain_head_for_path(&self, path: &str) -> Option<u32> {
        self.get_chain_for_path(path).into_iter().next()
    }
}

#[cfg(not(feature = "alloc"))]
pub use nop_mapper::*;
#[cfg(not(feature = "alloc"))]
pub type ClusterMapper = NopClusterMapper;
#[cfg(not(feature = "alloc"))]
mod nop_mapper {
    use super::*;
    use crate::fat::FatEntryValue;
    use core::str::from_utf8_unchecked;

    mod size_constants {
        pub const MAX_ENTRIES: usize = 1024;
        pub const MAX_CHAIN_LENGTH: usize = 1024;
        pub const MAX_PATH_LENGTH: usize = 1024;
    }

    pub struct NopClusterMapper {
        entries: [FileEntry; size_constants::MAX_ENTRIES],
    }

    #[derive(Copy, Clone)]
    struct FileEntry {
        path: [u8; size_constants::MAX_PATH_LENGTH],
        chain: [u32; size_constants::MAX_CHAIN_LENGTH],
    }

    impl FileEntry {
        pub fn from_path(path: &str) -> FileEntry {
            let mut retval = FileEntry::default();
            let path_bytes = path.as_bytes();
            for (idx, bt) in path_bytes.iter().enumerate() {
                retval.path[idx] = *bt;
            }
            retval
        }
        pub fn path_strlen(&self) -> usize {
            self.path.iter().take_while(|&&c| c != 0).count()
        }
        pub fn path_str(&self) -> &str {
            unsafe { from_utf8_unchecked(&self.path[0..self.path_strlen()]) }
        }

        pub fn chain_count(&self) -> usize {
            (&self.chain)
                .iter()
                .take_while(|&&c| FatEntryValue::from(c) != FatEntryValue::Bad)
                .count()
        }

        pub fn add_cluster(&mut self, cluster: u32) {
            self.chain[self.chain_count()] = cluster;
        }
    }

    impl Default for FileEntry {
        fn default() -> FileEntry {
            FileEntry {
                path: [0; size_constants::MAX_PATH_LENGTH],
                chain: [u32::max_value(); size_constants::MAX_CHAIN_LENGTH],
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct ChainIter {
        chain: [u32; size_constants::MAX_CHAIN_LENGTH],
        idx: usize,
    }

    impl Default for ChainIter {
        fn default() -> Self {
            ChainIter {
                chain: [FatEntryValue::End.into(); size_constants::MAX_CHAIN_LENGTH],
                idx: 0,
            }
        }
    }

    impl Iterator for ChainIter {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            if self.idx >= self.chain.len() {
                return None;
            }
            let itm = self.chain[self.idx];
            self.idx += 1;
            match FatEntryValue::from(itm) {
                FatEntryValue::Next(n) => Some(n),
                _ => None,
            }
        }
    }

    impl NopClusterMapper {
        fn find_path_entry(&self, path: &str) -> Option<usize> {
            let path_bytes = path.as_bytes();
            if path_bytes.len() > size_constants::MAX_PATH_LENGTH {
                return None;
            }
            (&self.entries)
                .iter()
                .enumerate()
                .find(|(_, ent)| (&ent.path[..path_bytes.len()]) == path_bytes)
                .map(|(idx, _)| idx)
        }

        fn find_cluster_entry(&self, cluster: u32) -> Option<(usize, usize)> {
            (&self.entries)
                .iter()
                .enumerate()
                .find_map(|(path_idx, ent)| {
                    let mut chain = (&ent.chain)
                        .iter()
                        .enumerate()
                        .take_while(|(_, c)| FatEntryValue::from(**c) != FatEntryValue::Bad);
                    let cluster_idx = chain.find(|(_, c)| **c == cluster);
                    match cluster_idx {
                        Some((cidx, _)) => Some((path_idx, cidx)),
                        None => None,
                    }
                })
        }

        fn entry_count(&self) -> usize {
            (&self.entries)
                .iter()
                .take_while(|e| e.path_strlen() > 0)
                .count()
        }
    }

    impl ClusterMapperOps for NopClusterMapper {
        type ChainIterator = ChainIter;

        fn new() -> Self {
            Self {
                entries: [Default::default(); size_constants::MAX_ENTRIES],
            }
        }
        fn get_path_for_cluster(&self, cluster: u32) -> Option<&str> {
            let (pidx, _) = self.find_cluster_entry(cluster)?;
            Some(self.entries[pidx].path_str())
        }
        fn get_chain_for_path(&self, path: &str) -> Self::ChainIterator {
            if let Some(ent_idx) = self.find_path_entry(path) {
                let ent = self.entries[ent_idx];
                ChainIter {
                    chain: ent.chain,
                    idx: 0,
                }
            } else {
                ChainIter {
                    chain: [FatEntryValue::Bad.into(); size_constants::MAX_CHAIN_LENGTH],
                    idx: 0,
                }
            }
        }
        fn add_cluster_to_path(&mut self, path: &str, cluster: u32) {
            let existing = self.find_path_entry(path);
            let entry = match existing {
                Some(eidx) => &mut self.entries[eidx],
                None => {
                    self.entries[self.entry_count()] = FileEntry::from_path(path);
                    &mut self.entries[self.entry_count()]
                }
            };
            entry.add_cluster(cluster);
        }

        fn is_allocated(&self, cluster: u32) -> bool {
            self.find_cluster_entry(cluster).is_some()
        }
    }
}
#[cfg(feature = "alloc")]
pub use alloc_mapper::*;
#[cfg(feature = "alloc")]
pub type ClusterMapper = AllocClusterMapper;
#[cfg(feature = "alloc")]
mod alloc_mapper {
    use super::*;

    #[cfg(feature = "std")]
    use std as alloc;

    use alloc::borrow::ToOwned;
    use alloc::collections::HashMap;
    use alloc::string::String;
    use alloc::vec::Vec;
    pub struct AllocClusterMapper {
        cluster_mapping: HashMap<u32, String>,
        path_mapping: HashMap<String, Vec<u32>>,
    }

    impl ClusterMapperOps for AllocClusterMapper {
        type ChainIterator = Vec<u32>;

        fn new() -> Self {
            AllocClusterMapper {
                cluster_mapping: HashMap::new(),
                path_mapping: HashMap::new(),
            }
        }
        fn get_path_for_cluster(&self, cluster: u32) -> Option<&str> {
            self.cluster_mapping.get(&cluster).map(|s| s.as_ref())
        }
        fn get_chain_for_path(&self, path: &str) -> Self::ChainIterator {
            self.path_mapping
                .get(path)
                .map_or(Vec::new(), |v| v.clone())
        }
        fn add_cluster_to_path(&mut self, path: &str, cluster: u32) {
            if !self.path_mapping.contains_key(path) {
                self.path_mapping.insert(path.to_owned(), Vec::new());
            }
            if let Some(v) = self.path_mapping.get_mut(path) {
                v.push(cluster);
            }
            self.cluster_mapping.insert(cluster, path.to_owned());
        }

        fn is_allocated(&self, cluster: u32) -> bool {
            self.cluster_mapping.contains_key(&cluster)
        }
    }
}
