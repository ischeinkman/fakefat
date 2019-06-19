use crate::bpb::BiosParameterBlock;

const BAD_ENTRY: u32 = 0x0FFFFFF7;
const END_OF_CHAIN: u32 = 0x0FFFFFFF;
const FREE_ENTRY: u32 = 0;

/// A single entry in the File Allocation Table, which corresponds to where
/// a reader would jump to after finishing the current cluster.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FatEntryValue {
    /// The current cluster is not part of any allocation.
    Free,

    /// The current cluster is part of an allocation chain, and there is another
    /// cluster after it in the chain at the given cluster index.
    Next(u32),

    /// Something is wrong with this cluster and/or chain.
    Bad,

    /// The current cluster is the final cluster in its chain.
    End,
}

impl From<u32> for FatEntryValue {
    fn from(inner: u32) -> FatEntryValue {
        match inner {
            FREE_ENTRY => FatEntryValue::Free,
            BAD_ENTRY => FatEntryValue::Bad,
            0x0FFFFFF8..=0x0FFFFFFF => FatEntryValue::End,
            n => FatEntryValue::Next(n),
        }
    }
}

impl From<FatEntryValue> for u32 {
    fn from(wrapped: FatEntryValue) -> u32 {
        match wrapped {
            FatEntryValue::Free => FREE_ENTRY,
            FatEntryValue::Bad => BAD_ENTRY,
            FatEntryValue::End => END_OF_CHAIN,
            FatEntryValue::Next(n) => n,
        }
    }
}

/// Converts a raw device offset to the index of the cluster whose entry is being
/// searched.
///
/// The `bpb` value is passed for the sake of the reserved byte count and FAT size.
pub fn idx_to_cluster(bpb: &BiosParameterBlock, idx: usize) -> u32 {
    let reserved_sectors = bpb.reserved_sectors as usize;
    let reserved_bytes = reserved_sectors * bpb.bytes_per_sector as usize;
    let fat_offset = (idx - reserved_bytes) % bpb.sectors_per_fat_32 as usize;
    let entry_cluster = fat_offset / 4;
    entry_cluster as u32
}
