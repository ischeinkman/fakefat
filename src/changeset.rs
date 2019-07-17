#![allow(unused)]

use crate::fat::FatEntryValue;

#[cfg(feature = "alloc")]
pub type ChangeSet = alloc_changeset::AllocChangeSet;
#[cfg(feature = "alloc")]
pub type ChangeBuff = alloc_changeset::AllocChangeBuff;

#[cfg(feature = "alloc")]
mod alloc_changeset {
    use super::*;
    #[cfg(feature = "std")]
    use std::collections::HashMap;
    #[cfg(feature = "std")]
    type Map<K, V> = HashMap<K, V>;

    #[cfg(not(feature = "std"))]
    extern crate alloc;
    #[cfg(not(feature = "std"))]
    use alloc::collections::BTreeMap;
    #[cfg(not(feature = "std"))]
    type Map<K, V> = BTreeMap<K, V>;

    #[derive(Clone)]
    pub struct AllocChangeBuff {
        data: Vec<u8>,
        entry: FatEntryValue,
    }

    impl ChangeSetEntry for AllocChangeBuff {
        fn data(&self) -> &[u8] {
            &self.data
        }
        fn entry(&self) -> FatEntryValue {
            self.entry
        }
    }

    pub struct AllocChangeSet {
        entries: Map<u32, AllocChangeBuff>,
        cluster_size: usize,
    }

    impl AllocChangeSet {
        pub fn entries<'a>(&'a self) -> impl Iterator<Item = (u32, AllocChangeBuff)> + 'a {
            self.entries.iter().map(|(&k, v)| (k, v.clone()))
        }
    }

    impl ChangeSetOps for AllocChangeSet {
        fn new(cluster_size: u32) -> Self {
            AllocChangeSet {
                entries: Map::new(),
                cluster_size: cluster_size as usize,
            }
        }

        fn cluster_entry(&self, cluster: u32) -> Option<FatEntryValue> {
            self.entries.get(&cluster).map(|ent| ent.entry)
        }

        fn set_cluster_entry(&mut self, cluster: u32, new_entry: FatEntryValue) {
            let itm_ref = self.entries.get_mut(&cluster).unwrap();
            (*itm_ref).entry = new_entry;
        }

        fn cluster_data(&self, cluster: u32) -> Option<&[u8]> {
            self.entries.get(&cluster).map(|ent| ent.data.as_ref())
        }

        fn cluster_mut(&mut self, cluster: u32) -> Option<&mut [u8]> {
            self.entries.get_mut(&cluster).map(|ent| ent.data.as_mut())
        }

        fn insert_cluster(&mut self, cluster: u32, entry: FatEntryValue) -> &mut [u8] {
            let data = vec![0; self.cluster_size];
            let new_change_item = AllocChangeBuff { data, entry };
            self.entries.insert(cluster, new_change_item);
            &mut self.entries.get_mut(&cluster).unwrap().data
        }
    }
}

#[cfg(not(feature = "alloc"))]
pub type ChangeSet = noalloc_changeset::NoallocChangeSet;
#[cfg(not(feature = "alloc"))]
pub type ChangeBuff = noalloc_changeset::NoallocChangeBuff;

#[cfg(not(feature = "alloc"))]
mod noalloc_changeset {
    use super::*;
    const CLUSTER_BUFFER_SIZE: usize = 1024 * 4;
    const CHANGESET_CAPACITY: usize = 1024;

    #[derive(Clone, Copy)]
    pub struct NoallocChangeBuff {
        cluster: u32,
        data: [u8; CLUSTER_BUFFER_SIZE],
        entry: FatEntryValue,
    }

    impl Default for NoallocChangeBuff {
        fn default() -> Self {
            NoallocChangeBuff {
                cluster: FatEntryValue::Bad.into(),
                data: [0; CLUSTER_BUFFER_SIZE],
                entry: FatEntryValue::Free,
            }
        }
    }

    impl ChangeSetEntry for NoallocChangeBuff {
        fn entry(&self) -> FatEntryValue {
            self.entry
        }
        fn data(&self) -> &[u8] {
            &self.data
        }
    }

    pub struct NoallocChangeIter<'a> {
        idx: usize,
        changes: &'a [NoallocChangeBuff],
    }

    impl<'a> NoallocChangeIter<'a> {
        pub fn new(changes: &'a [NoallocChangeBuff]) -> Self {
            Self { changes, idx: 0 }
        }
    }

    impl<'a> Iterator for NoallocChangeIter<'a> {
        type Item = (u32, NoallocChangeBuff);

        fn next(&mut self) -> Option<Self::Item> {
            let retval = self
                .changes
                .get(self.idx)
                .copied()
                .filter(|ent| ent.entry() != FatEntryValue::Bad)
                .map(|ent| (ent.cluster, ent));
            if retval.is_some() {
                self.idx += 1;
            }
            retval
        }
    }

    pub struct NoallocChangeSet {
        changes: [NoallocChangeBuff; CHANGESET_CAPACITY],
    }

    impl NoallocChangeSet {
        pub fn entries<'a>(&'a self) -> impl Iterator<Item = (u32, NoallocChangeBuff)> + 'a {
            NoallocChangeIter::new(&self.changes)
        }
    }

    impl ChangeSetOps for NoallocChangeSet {
        fn new(_cluster_size: u32) -> Self {
            NoallocChangeSet {
                changes: [Default::default(); CHANGESET_CAPACITY],
            }
        }

        fn cluster_entry(&self, cluster: u32) -> Option<FatEntryValue> {
            let idx = self
                .changes
                .binary_search_by_key(&cluster, |buff| buff.cluster)
                .ok()?;
            Some(self.changes[idx].entry)
        }

        fn set_cluster_entry(&mut self, cluster: u32, new_entry: FatEntryValue) {
            if let Ok(idx) = self
                .changes
                .binary_search_by_key(&cluster, |buff| buff.cluster)
            {
                self.changes[idx].entry = new_entry;
            }
        }

        fn cluster_data(&self, cluster: u32) -> Option<&[u8]> {
            let idx = self
                .changes
                .binary_search_by_key(&cluster, |buff| buff.cluster)
                .ok()?;
            Some(&self.changes[idx].data)
        }

        fn cluster_mut(&mut self, cluster: u32) -> Option<&mut [u8]> {
            let idx = self
                .changes
                .binary_search_by_key(&cluster, |buff| buff.cluster)
                .ok()?;
            Some(&mut self.changes[idx].data)
        }
        fn insert_cluster(&mut self, cluster: u32, entry: FatEntryValue) -> &mut [u8] {
            if let Ok(idx) = self
                .changes
                .binary_search_by_key(&cluster, |buff| buff.cluster)
            {
                &mut self.changes[idx].data
            } else {
                let free_idx = self
                    .changes
                    .binary_search_by_key(&FatEntryValue::Bad.into(), |buff| buff.cluster)
                    .unwrap();
                self.changes[free_idx].cluster = cluster;
                self.changes[free_idx].entry = entry;
                self.changes.sort_unstable_by_key(|buff| buff.cluster);
                self.cluster_mut(cluster).unwrap()
            }
        }
    }
}

pub trait ChangeSetOps {
    fn new(cluster_size: u32) -> Self;

    fn cluster_entry(&self, cluster: u32) -> Option<FatEntryValue>;

    fn set_cluster_entry(&mut self, cluster: u32, new_entry: FatEntryValue);

    fn cluster_data(&self, cluster: u32) -> Option<&[u8]>;

    fn cluster_mut(&mut self, cluster: u32) -> Option<&mut [u8]>;
    fn insert_cluster(&mut self, cluster: u32, entry: FatEntryValue) -> &mut [u8];

    // Rust doesn't yet allow for `impl Trait` as part of a trait definition,
    // so since this is trait only really exists for easier compile time checks that
    // the noalloc and alloc version matches up we can just cheat by moving this to a
    // struct impl.
    // type EntryType : ChangeSetEntry
    // fn changes(&self) -> impl Iterator<Item = (u32, Self::EntryType)>;
}

pub trait ChangeSetEntry {
    fn data(&self) -> &[u8];
    fn entry(&self) -> FatEntryValue;
}
