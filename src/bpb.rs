use super::ReadByte;

const FAT_32_LABEL: [u8; 8] = [b'F', b'A', b'T', b'3', b'2', b' ', b' ', b' '];
const FAT_COUNT: u8 = 2;
const RESERVED_SECTORS: u16 = 8;
const MEDIA: u8 = 0xf8;
const SECTORS_PER_TRACK: u16 = 32; //WHY?
const ROOT_DIR_FIRST_CLUSTER: u32 = 2;
const HEADS: u16 = 64; //WHY?
const BACKUP_BOOT_SECTOR: u16 = 6; //See above
const DRIVE_NUM: u8 = 0x80; //Endpoint related?

/// Represents the metadata present at the head of every FAT32 filesystem.
///
/// While it is possible to create one by hand, the values provided by
/// `BiosParameterBlock::from_sector_information` should suffice for most use cases; generally it is recommended
/// to use the default as a base and modify specific fields instead of creating the
/// entire preamble from scratch.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BiosParameterBlock {
    /// The number of bytes that the virtual "backing device" reads and writes
    /// at a time; defaults to 512.
    pub bytes_per_sector: u16,

    /// The number of "device sectors" that each of the fake FAT clusters represents;
    /// defaults to 8.
    pub sectors_per_cluster: u8,

    /// The number of sectors which are set aside for the File Allocation Tables and
    /// preamble. Defaults to 8, since we want to round to the nearest cluster count.
    pub reserved_sectors: u16,

    /// The number of mirrored File Allocation Tables to use in this fake filesystem;
    /// defaults to 2 since many hosts only support that number.
    pub fats: u8,

    /// Not sure; defaults to 0xf8.
    pub media: u8,
    /// Not sure; defaults to 32.
    pub sectors_per_track: u16,
    /// Not sure; defaults to 64.
    pub heads: u16,
    /// Not sure; defaults to 0. 
    pub hidden_sectors: u32,

    /// The size of the filesystem in sectors, including all FATs and the preamble.
    pub total_sectors_32: u32,

    /// The number of sectors that a single File Allocation Table uses.
    /// By default calculated using `default_sectors_per_fat`.
    pub sectors_per_fat_32: u32,

    /// Extra filesystem flags. 
    /// 
    /// Currently only the mirroring flag bit (`0x80`) is used by this crate.
    pub extended_flags: u16,

    /// The first cluster of the root directory, usually equal to `reserved_sectors/sectors_per_cluster + 1`.
    pub root_dir_first_cluster: u32,

    /// The sector to find the informational struct containing information about
    /// the free clusters.
    pub fs_info_sector: u16,

    /// Not sure; defaults to 6. 
    /// 
    /// Since the first 8 sectors are allocated as the filesystem header, this 
    /// may be a copy of the raw BIOS bytes that are located at the head of all
    /// single-partition SCSI drive, but this is not yet confirmed. 
    pub backup_boot_sector: u16,
    /// Not sure; defaults to `0x80`.  
    pub drive_num: u8,
    /// Not sure; defaults to 0. 
    pub volume_id: u32,

    /// The label of this filesystem volume.
    pub volume_label: [u8; 11],

    /// The current location of the filesystem for the purposes of `Read`/`Write`/`Seek`.
    pub read_idx: usize,
}

impl Default for BiosParameterBlock {
    fn default() -> BiosParameterBlock {
        BiosParameterBlock {
            bytes_per_sector: 512,
            sectors_per_cluster: 8,
            reserved_sectors: RESERVED_SECTORS,
            fats: FAT_COUNT,
            media: MEDIA,
            sectors_per_track: SECTORS_PER_TRACK,
            heads: HEADS,
            hidden_sectors: 0,
            total_sectors_32: 0,
            sectors_per_fat_32: 0,

            extended_flags: 0,
            root_dir_first_cluster: ROOT_DIR_FIRST_CLUSTER,
            fs_info_sector: 1,
            backup_boot_sector: BACKUP_BOOT_SECTOR,
            drive_num: DRIVE_NUM,
            volume_id: 0,
            volume_label: [0; 11],
            read_idx: 0,
        }
    }
}

impl ReadByte for BiosParameterBlock {
    const SIZE: usize = 512;
    fn read_byte(&self, idx: usize) -> u8 {
        if idx < 11 {
            return b'a';
        } else if idx == 510 {
            return 0x55;
        } else if idx == 511 {
            return 0xaa;
        }
        let idx = idx - 11;
        match idx {
            0 => ((self.bytes_per_sector & 0xFF) as u8),
            1 => (((self.bytes_per_sector >> 8) & 0xFF) as u8),
            2 => self.sectors_per_cluster,
            3 => (self.reserved_sectors & 0xFF) as u8,
            4 => ((self.reserved_sectors >> 8) & 0xFF) as u8,
            5 => self.fats,
            6 => 0, //(self.root_entries & 0xFF) as u8,
            7 => 0, // ((self.root_entries >> 8) & 0xFF) as u8,
            8 => 0, // (self.total_sectors_16 & 0xFF) as u8,
            9 => 0, //((self.total_sectors_16 >> 8) & 0xFF) as u8,
            10 => self.media,
            11 => 0, // (self.sectors_per_fat_16 & 0xFF) as u8,
            12 => 0, //((self.sectors_per_fat_16 >> 8) & 0xFF) as u8,
            13 => (self.sectors_per_track & 0xFF) as u8,
            14 => ((self.sectors_per_track >> 8) & 0xFF) as u8,
            15 => (self.heads & 0xFF) as u8,
            16 => ((self.heads >> 8) & 0xFF) as u8,
            17 => (self.hidden_sectors & 0xFF) as u8,
            18 => ((self.hidden_sectors >> 8) & 0xFF) as u8,
            19 => ((self.hidden_sectors >> 16) & 0xFF) as u8,
            20 => ((self.hidden_sectors >> 24) & 0xFF) as u8,
            21 => (self.total_sectors_32 & 0xFF) as u8,
            22 => ((self.total_sectors_32 >> 8) & 0xFF) as u8,
            23 => ((self.total_sectors_32 >> 16) & 0xFF) as u8,
            24 => ((self.total_sectors_32 >> 24) & 0xFF) as u8,

            25 => (self.sectors_per_fat_32 & 0xFF) as u8,
            26 => ((self.sectors_per_fat_32 >> 8) & 0xFF) as u8,
            27 => ((self.sectors_per_fat_32 >> 16) & 0xFF) as u8,
            28 => ((self.sectors_per_fat_32 >> 24) & 0xFF) as u8,
            29 => (self.extended_flags & 0xFF) as u8,
            30 => ((self.extended_flags >> 8) & 0xFF) as u8,
            31 => 0, //(self.fs_version & 0xFF) as u8,
            32 => 0, //((self.fs_version >> 8) & 0xFF) as u8,
            33 => (self.root_dir_first_cluster & 0xFF) as u8,
            34 => ((self.root_dir_first_cluster >> 8) & 0xFF) as u8,
            35 => ((self.root_dir_first_cluster >> 16) & 0xFF) as u8,
            36 => ((self.root_dir_first_cluster >> 24) & 0xFF) as u8,
            37 => (self.fs_info_sector & 0xFF) as u8,
            38 => ((self.fs_info_sector >> 8) & 0xFF) as u8,
            39 => (self.backup_boot_sector & 0xFF) as u8,
            40 => ((self.backup_boot_sector >> 8) & 0xFF) as u8,
            _b @ 41..=52 => 0, // self.reserved_0[b - 41],
            53 => self.drive_num,
            54 => 0,    //self.reserved_1,
            55 => 0x29, //self.ext_sig,
            56 => (self.volume_id & 0xFF) as u8,
            57 => ((self.volume_id >> 8) & 0xFF) as u8,
            58 => ((self.volume_id >> 16) & 0xFF) as u8,
            59 => ((self.volume_id >> 24) & 0xFF) as u8,
            b @ 60..=70 => self.volume_label[b - 60],
            b @ 71..=78 => FAT_32_LABEL[b - 71], //self.fs_type_label[b - 71],
            //79 => 0xaa,
            //80 => 0x55,
            _b => 0,
        }
    }
}

impl BiosParameterBlock {

    /// Constructs a new `BiosParameterBlock` with the given values for 
    /// `total_sectors` and `bytes_per_sector` and default values for everything else. 
    /// 
    /// The value of `sectors_per_fat_32` is calculated via the `default_sectors_per_fat`
    /// function and the provided values. 
    pub fn from_sector_information(
        total_sectors: u32,
        bytes_per_sector: u16,
    ) -> BiosParameterBlock {
        let mut retval = BiosParameterBlock::default();
        retval.bytes_per_sector = bytes_per_sector;
        retval.total_sectors_32 = total_sectors;
        let spf = default_sectors_per_fat(&retval);
        retval.sectors_per_fat_32 = spf;
        retval
    }

    /// Assuming a preamble with more than 1 File Allocation Table, returns whether
    /// writes to 1 FAT are automatically duplicated across all other FATs. 
    pub fn is_mirroring_enabled(&self) -> bool {
        self.extended_flags & 0x80 == 0
    }

    /// The number of bytes each cluster spans in the fake File Allocation Table.
    ///
    /// In a normal FAT32 filesystem, all files smaller than a single cluster
    /// would still take up this many bytes on disk, since the File Allocation Table
    /// cannot more granularly allocate the disk space.
    pub fn bytes_per_cluster(&self) -> u32 {
        u32::from(self.bytes_per_sector) * 
        u32::from(self.sectors_per_cluster)
    }
    
    /// Returns the starting address of the first File Allocation Table. 
    pub fn fat_start(&self) -> usize {
        self.reserved_sectors as usize * self.bytes_per_sector as usize
    }
    
    /// Returns the first index after the end of the final File Allocation Table. 
    pub fn fat_end(&self) -> usize {
        self.fat_start()
            + (self.fats as usize)
                * (self.sectors_per_fat_32 as usize)
                * (self.bytes_per_sector as usize)
    }
}

/// Calculates a sane default to use for the size of each File Allocation Table
/// based on the values of the passed in preamble.
pub fn default_sectors_per_fat(bpb: &BiosParameterBlock) -> u32 {
    // Adapted from the fatfs crate.
    // Not completely sure how it works to be honest. TODO: Figure that out.
    let not_reserved = bpb.total_sectors_32 - u32::from(bpb.reserved_sectors);
    let t1: u64 = u64::from(not_reserved) + u64::from(2 * u32::from(bpb.sectors_per_cluster));
    let bytes_per_cluster = u32::from(bpb.sectors_per_cluster) * u32::from(bpb.bytes_per_sector);
    let t2 = u64::from(bytes_per_cluster / 4 + u32::from(bpb.fats));
    let sectors_per_fat = (t1 + t2 - 1) / t2;
    sectors_per_fat as u32
}
