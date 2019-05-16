use crate::dirent::{FileDirEntry, LfnDirEntry};
use crate::shortname::ShortName;
use alloc::vec::Vec;

pub fn lfn_count_for_name(name: &str) -> usize {
    if ShortName::from_str(name).is_some() {
        return 0;
    }
    name.len() / 13 + if name.len() % 13 != 0 { 1 } else { 0 }
}

pub fn construct_name_entries(name: &str, base: FileDirEntry) -> Vec<LfnDirEntry> {
    let mut retval = Vec::new();
    if name == base.name.to_string() {
        return retval;
    }

    let checksum = base.name.lfn_checksum();
    let entries_len = name.len() / 13 + if name.len() % 13 != 0 { 1 } else { 0 };
    retval.reserve(entries_len);
    for (idx, part) in name.as_bytes().chunks(13).enumerate() {
        let mut newent = LfnDirEntry::default();
        newent.entry_num = idx as u8;
        newent.checksum = checksum;

        let part_len = part.len();
        (&mut newent.name_part[..part_len]).copy_from_slice(part);
        retval.push(newent);
    }
    retval[0].entry_num |= 0x40;
    retval
}
