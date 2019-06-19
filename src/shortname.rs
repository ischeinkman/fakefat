use core::cmp;
use core::str::from_utf8_unchecked;

use super::ReadByte;

/// Represents a single name allowable in a normal directory entry, which is
/// an 8 ASCII character name and a 3 ASCII character extention.
#[derive(Copy, Clone, Debug)]
pub struct ShortName {
    /// The characters in this name.
    pub data: [u8; 11],

    /// Whether the non-extension portion is lowercase.
    pub lower_name: bool,

    /// Whether the extension portion is lowercase.
    pub lower_ext: bool,
}

impl Default for ShortName {
    fn default() -> Self {
        ShortName {
            data: [b' '; 11],
            lower_name: false,
            lower_ext: false,
        }
    }
}

impl PartialEq<ShortName> for ShortName {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.ext() == other.ext()
    }
}

impl Eq for ShortName {}

impl PartialOrd for ShortName {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.to_str().partial_cmp(&other.to_str())
    }
}
impl Ord for ShortName {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.to_str().cmp(&other.to_str())
    }
}

impl ReadByte for ShortName {
    const SIZE: usize = ShortName::SHORT_NAME_FULL_LENGTH;

    fn read_byte(&self, idx: usize) -> u8 {
        if idx == 0 && self.data[0] == 0xE5 {
            0x05
        } else if idx < self.data.len() {
            self.data[idx]
        } else {
            0
        }
    }
}

impl ShortName {

    /// The maximum length of the name section of a FAT32 ShortName. 
    pub const SHORT_NAME_LENGTH: usize = 8;
    /// The maximum length of the extension section of a FAT32 ShortName. 
    pub const SHORT_NAME_EXT_LENGTH: usize = 3;
    /// The maximum length of a FAT32 ShortName. 
    pub const SHORT_NAME_FULL_LENGTH: usize = Self::SHORT_NAME_EXT_LENGTH + Self::SHORT_NAME_LENGTH;

    /// The length of the non-extension portion of this `ShortName`.
    pub fn name_len(self) -> usize {
        (&self.data[..8])
            .iter()
            .take_while(|&&c| !is_end_marker(c.into()))
            .count()
    }

    /// The length of the extension portion of this `ShortName`.
    pub fn ext_len(self) -> usize {
        (&self.data[8..])
            .iter()
            .take_while(|&&c| !is_end_marker(c.into()))
            .count()
    }

    /// The non-extention portion of this `ShortName`.
    pub fn name(&self) -> &str {
        unsafe { from_utf8_unchecked(&self.data[..self.name_len()]) }
    }

    /// The extention portion of this `ShortName`.
    pub fn ext(&self) -> &str {
        unsafe { from_utf8_unchecked(&self.data[8..8 + self.ext_len()]) }
    }

    /// Returns the FAT32 flag byte for this `ShortName`'s cases. 
    pub fn case_flag(self) -> u8 {
        match (self.lower_name, self.lower_ext) {
            (true, true) => 0x18,
            (true, false) => 0x08,
            (false, true) => 0x10,
            (false, false) => 0x0,
        }
    }

    /// Converts the **raw** shortname into a `&str`. 
    /// 
    /// This means that the returned value will always be exactly 11 ASCII capital,
    /// with both the name and extension portion being padded by spaces. 
    pub fn to_str(&self) -> &str {
        unsafe { from_utf8_unchecked(&self.data) }
    }

    /// Attempts to create a `ShortName` out of the passed in `name`.
    ///
    /// Returns `None` if `name` is not a valid `ShortName`, which could be for
    /// any of the following reasons:
    ///
    /// *  The non-extension portion of `name` is longer than 8 characters.
    /// *  The extension portion of `name` is longer than 3 characters.
    /// *  The non-extension portion of `name` does not all have the same case.
    /// *  The extension portion of `name` does not all have the same case.
    /// *  Any of the characters is not one in the list allowed by the FAT filesystem spec.
    pub fn from_str<T: AsRef<str>>(name: T) -> Option<ShortName> {
        let name: &str = name.as_ref();
        if name.len() > ShortName::SHORT_NAME_FULL_LENGTH || name.is_empty() {
            return None;
        }

        let mut retval = ShortName::default();

        let mut ext_idx = name.len();
        let mut name_case = 0;
        for (idx, c) in name.char_indices() {
            let case = case_val(c);
            if idx > 7 || !is_valid_char(c) || name_case + case == 3 {
                return None;
            } else if is_end_marker(c) {
                ext_idx = idx;
                break;
            } else if name_case == 0 && case != 0 {
                name_case = case;
                retval.lower_name = case == 1;
            }

            retval.data[idx] = char_to_byte(c);
        }
        if ext_idx == 0 {
            return None;
        } else if ext_idx == name.len() {
            return Some(retval);
        }
        let mut ext_case = 0;
        for (idx, c) in name.char_indices().skip(ext_idx + 1) {
            let idx = idx - ext_idx - 1;
            let case = case_val(c);
            if idx > 2 || !is_valid_char(c) || name_case + case == 3 {
                return None;
            } else if is_end_marker(c) {
                break;
            } else if ext_case == 0 && case != 0 {
                ext_case = case;
                retval.lower_ext = case == 1;
            }

            retval.data[idx + 8] = char_to_byte(c);
        }
        Some(retval)
    }

    /// Converts a passed in `name` to a ShortName, hashing the long name if it
    /// is not valid. `duplicate_count` represents the offset to add to the hash,
    /// for use when we expect a collision between multiple long names.
    pub fn convert_str<T: AsRef<str>>(name: T, duplicate_count: u8) -> ShortName {
        let mut retval = ShortName::default();

        let name: &str = name.as_ref();
        if let Some(r) = ShortName::from_str(name) {
            return r;
        }
        let ext_idx = name
            .char_indices()
            .rfind(|(_, c)| *c == '.')
            .map(|(idx, _)| idx);
        let (name_part_raw, ext_part_raw) = ext_idx.map_or((name, ""), |idx| name.split_at(idx));
        let name_part = to_valid_shortname(name_part_raw);
        let mut name_part_idx = 0;
        for c in name_part {
            retval.data[name_part_idx] = char_to_byte(c);
            name_part_idx += 1;
            if name_part_idx > 7 {
                break;
            }
        }
        let ext_part = to_valid_shortname(ext_part_raw);
        let mut ext_part_idx = 0;
        for c in ext_part {
            retval.data[ext_part_idx + 8] = char_to_byte(c);
            ext_part_idx += 1;
            if ext_part_idx + 8 >= retval.data.len() {
                break;
            }
        }
        if duplicate_count == 0 {
            retval.data[6] = b'~';
            retval.data[7] = b'~';
        } else {
            let mut suffix_digits_left = duplicate_count;
            let mut cur_idx = 7;
            while suffix_digits_left > 0 {
                let digit = suffix_digits_left % 10;
                let digit_char = digit + b'0';
                retval.data[cur_idx] = digit_char;
                cur_idx -= 1;
                suffix_digits_left /= 10;
            }
            retval.data[cur_idx] = b'~';
        }
        retval
    }

    /// Calculates a checksum from this `ShortName` to associate it with a series
    /// of Long Name entries.
    pub fn lfn_checksum(&self) -> u8 {
        let mut retval: u8 = 0;
        for c in self.data.iter() {
            let shifted_retval = ((retval & 1) << 7) + ((retval & 0xFE) >> 1);
            retval = shifted_retval.wrapping_add(*c);
        }

        retval
    }
}

fn char_to_byte(assumed_valid: char) -> u8 {
    let mut tmpbuff = [0; 1];
    assumed_valid.encode_utf8(&mut tmpbuff);
    tmpbuff[0]
}

fn is_valid_char(inp: char) -> bool {
    inp.len_utf8() == 1
        && (inp.is_ascii_uppercase()
            || inp.is_ascii_lowercase()
            || inp.is_ascii_digit()
            || is_end_marker(inp)
            || inp == '!'
            || inp == '@'
            || inp == '#'
            || inp == '$'
            || inp == '%'
            || inp == '^'
            || inp == '&'
            || inp == '('
            || inp == ')'
            || inp == '{'
            || inp == '}')
}

fn is_end_marker(inp: char) -> bool {
    inp == ' ' || inp == '.' || inp == '\0'
}

fn case_val(inp: char) -> u8 {
    if inp.is_ascii_lowercase() {
        1
    } else if inp.is_ascii_uppercase() {
        2
    } else {
        0
    }
}

fn to_valid_shortname<'a>(raw: &'a str) -> impl Iterator<Item = char> + 'a {
    raw.chars().filter_map(|c| {
        if is_end_marker(c) {
            None
        } else if !is_valid_char(c) {
            Some('_')
        } else {
            Some(c.to_ascii_uppercase())
        }
    })
}
