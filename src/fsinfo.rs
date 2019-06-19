use crate::ReadByte;

/// The second part of the FAT filesystem preamble, containing information
/// about the free space in the filesystem.
pub struct FsInfoSector {
    free_count: u32,
    next_free: u32,
}

impl Default for FsInfoSector {
    fn default() -> FsInfoSector {
        FsInfoSector {
            free_count: 0xFFFF_FFFF,
            next_free: 0xFFFF_FFFF,
        }
    }
}

impl ReadByte for FsInfoSector {
    const SIZE: usize = 512;

    fn read_byte(&self, idx: usize) -> u8 {
        match idx {
            0 => 0x52,
            1 => 0x52,
            2 => 0x61,
            3 => 0x41,

            484 => 0x72,
            485 => 0x72,
            486 => 0x41,
            487 => 0x61,

            488 => (self.free_count & 0xFF) as u8,
            489 => ((self.free_count >> 8) & 0xFF) as u8,
            490 => ((self.free_count >> 16) & 0xFF) as u8,
            491 => ((self.free_count >> 24) & 0xFF) as u8,
            492 => (self.next_free & 0xFF) as u8,
            493 => ((self.next_free >> 8) & 0xFF) as u8,
            494 => ((self.next_free >> 16) & 0xFF) as u8,
            495 => ((self.next_free >> 24) & 0xFF) as u8,
            510 => 0x55,
            511 => 0xaa,
            _ => 0,
        }
    }
}
