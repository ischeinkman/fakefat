use std::io::{Read, self, Seek, SeekFrom};


const FAT_32_LABEL : [u8 ; 8] = [b'F', b'A', b'T', b'3', b'2', b' ', b' ', b' '];
const FAT_COUNT : u8 = 2;
const RESERVED_SECTORS : u16 = 8;
const MEDIA : u8 = 0xf8;
const SECTORS_PER_TRACK : u16 = 32; //WHY?
const ROOT_DIR_FIRST_CLUSTER : u32 = 2;
const HEADS : u16 = 64; //WHY?
const BACKUP_BOOT_SECTOR : u16 = 6; //See above
const DRIVE_NUM : u8 = 0x80; //Endpoint related?

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BiosParameterBlock {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors : u16, 
    pub fats : u8, 
    pub media : u8, 
    pub sectors_per_track : u16, 
    pub heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub sectors_per_fat_32 : u32, 

    // Extended BIOS Parameter Block
    pub extended_flags: u16,
    pub root_dir_first_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub drive_num: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],

    pub read_idx : usize, 
}

impl Default for BiosParameterBlock {
    fn default() -> BiosParameterBlock {
        BiosParameterBlock {
            bytes_per_sector : 512, 
            sectors_per_cluster : 8,
            reserved_sectors : RESERVED_SECTORS,
            fats : FAT_COUNT,
            media : MEDIA,
            sectors_per_track : SECTORS_PER_TRACK,
            heads : HEADS,
            hidden_sectors : 0, 
            total_sectors_32 : 0,
            sectors_per_fat_32 : 0,  

            extended_flags : 0,  
            root_dir_first_cluster : ROOT_DIR_FIRST_CLUSTER, 
            fs_info_sector : 1, 
            backup_boot_sector : BACKUP_BOOT_SECTOR, 
            drive_num : DRIVE_NUM,
            volume_id : 0, 
            volume_label : [0 ; 11],
            read_idx : 0,  
        }
    }
}

impl BiosParameterBlock {
    pub fn from_sector_information(total_sectors : u32, bytes_per_sector : u16) -> BiosParameterBlock {
        let mut retval = BiosParameterBlock::default();
        retval.bytes_per_sector = bytes_per_sector;
        retval.total_sectors_32 = total_sectors;
        let spf = default_sectors_per_fat(&retval);
        retval.sectors_per_fat_32 = spf; 
        retval 
    }
    pub fn read_byte(&self, idx : usize) -> u8 {
        let retval = match idx {
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
            _b @ 41 ..= 52 => 0,// self.reserved_0[b - 41],
            53 => self.drive_num,
            54 => 0, //self.reserved_1,
            55 => 0x29, //self.ext_sig,
            56 => (self.volume_id & 0xFF) as u8,
            57 => ((self.volume_id >> 8) & 0xFF) as u8,
            58 => ((self.volume_id >> 16) & 0xFF) as u8,
            59 => ((self.volume_id >> 24) & 0xFF) as u8,
            b @ 60..=70 => self.volume_label[b - 60],
            b @ 71 ..=78 => FAT_32_LABEL[b - 71], //self.fs_type_label[b - 71], 
            b => {
                eprintln!("Trying to read past end: {}", b);
                0
            },        
        };

        dbg!(format!("Reading from position: {} => {:x}", idx, retval));
        retval
    }

}
fn default_sectors_per_fat(bpb : &BiosParameterBlock) -> u32 {
    // Adapted from the fatfs crate. 
    // Not completely sure how it works to be honest. TODO: Figure that out.
    let not_reserved = bpb.total_sectors_32 - bpb.reserved_sectors as u32; 
    let t1: u64 = u64::from(not_reserved) + u64::from(2 * u32::from(bpb.sectors_per_cluster));
    let bytes_per_cluster = bpb.sectors_per_cluster as u32 * bpb.bytes_per_sector as u32;
    let t2 = u64::from(bytes_per_cluster / 4 + u32::from(bpb.fats));
    let sectors_per_fat = (t1 + t2 - 1) / t2;
    sectors_per_fat as u32
}

impl Read for BiosParameterBlock {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let mut offset = 0;
        while offset + self.read_idx < 81 && offset < buf.len() {
            buf[offset] = self.read_byte(offset + self.read_idx);
            offset += 1;
        }
        self.read_idx += offset; 
        Ok(offset)
    }
}

impl Seek for BiosParameterBlock {
    fn seek(&mut self, pos : SeekFrom) -> Result<u64, io::Error> {
        match pos {
            SeekFrom::Start(abs) => {
                self.read_idx = abs as usize;
            },
            SeekFrom::End(back) => {
                let abs = 78 - (back.abs() as usize);
                self.read_idx = abs;
            },
            SeekFrom::Current(off) => {
                if off < 0 {
                    self.read_idx -= off.abs() as usize;
                }
                else {
                    self.read_idx += off.abs() as usize;
                }
            }
        }
        Ok(self.read_idx as u64)
    }
}
