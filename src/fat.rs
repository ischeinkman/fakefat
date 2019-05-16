use crate::bpb::BiosParameterBlock;

const BAD_ENTRY: u32 = 0x0FFFFFF7;
const END_OF_CHAIN: u32 = 0x0FFFFFFF;
const FREE_ENTRY: u32 = 0;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FatEntryValue {
    Free,
    Next(u32),
    Bad,
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

pub fn idx_to_cluster(bpb: &BiosParameterBlock, idx: usize) -> u32 {
    let reserved_sectors = bpb.reserved_sectors as usize;
    let reserved_bytes = reserved_sectors * bpb.bytes_per_sector as usize;
    let fat_offset = (idx - reserved_bytes) % bpb.sectors_per_fat_32 as usize;
    let entry_cluster = fat_offset / 4;
    entry_cluster as u32
}
