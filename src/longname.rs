use crate::dirent::{FileDirEntry, LfnDirEntry};
use crate::shortname::ShortName;

pub fn lfn_count_for_name(name: &str) -> usize {
    if ShortName::from_str(name).is_some() {
        return 0;
    }
    name.len() / 13 + if name.len() % 13 != 0 { 1 } else { 0 }
}

pub fn construct_name_entries<EntryType : From<LfnDirEntry>, BuffType : AsMut<[EntryType]>> (name: &str, base: FileDirEntry, mut allocation : BuffType) {
    let entries_len = lfn_count_for_name(name);
    if entries_len == 0 {
        return;
    }
    let buff = allocation.as_mut();
    let checksum = base.name.lfn_checksum();
    let entries_len = lfn_count_for_name(name);
    debug_assert!(entries_len > 0, "Got count-entry mismatch: {} for {}.", entries_len, name);
    debug_assert!(entries_len <= buff.len(), "Bad allocation: needed {} entries but only got buffer length {}.", entries_len, buff.len());

    for (idx, part) in name.as_bytes().chunks(13).enumerate() {
        let mut newent = LfnDirEntry::default();
        newent.entry_num = if idx == 0 { 0x40 | (entries_len as u8) } else { entries_len as u8 - idx as u8} ;
        newent.checksum = checksum;

        let part_len = part.len();
        (&mut newent.name_part[..part_len]).copy_from_slice(part);
        buff[idx] = newent.into();
    }
}
