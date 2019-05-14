#[derive(Copy, Clone, Debug)]
pub struct ShortName {
    pub name : [u8 ; 8],
    pub ext : [u8 ; 3],
    pub lower_name : bool, 
    pub lower_ext : bool, 
}

impl Default for ShortName {
    fn default() -> Self {
        ShortName {
            name : [b' ' ; 8],
            ext : [b' ' ; 3],
            lower_name : false, 
            lower_ext : false, 
        }
    }
}

impl PartialEq<ShortName> for ShortName {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.ext  == other.ext
    }
}

impl Eq for ShortName {}

use std::fmt;
use std::str::from_utf8_unchecked;
impl fmt::Display for ShortName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name_len = self.name.iter().rposition(|b| *b != b' ' && *b != 0).unwrap_or(0);
        let name_part = if name_len > 0 { unsafe { from_utf8_unchecked(&self.name[..name_len]) }} else { "{NONE}"};
        let ext_len = self.ext.iter().rposition(|b| *b != b' ' && *b != 0).unwrap_or(0);
        let ext_part = if ext_len > 0 { unsafe { from_utf8_unchecked(&self.ext[..ext_len]) }} else { "{NONE}"};
        write!(f, "ShortName{{ name: {}, ext : {} }}", name_part, ext_part)
    }
}

impl ShortName {
    pub const SHORT_NAME_LENGTH : usize = 8;
    pub const SHORT_NAME_EXT_LENGTH : usize = 3;
    pub const SHORT_NAME_FULL_LENGTH : usize = Self::SHORT_NAME_EXT_LENGTH + Self::SHORT_NAME_LENGTH;

    pub fn read_byte(self, idx : usize) -> u8 {
        match idx {
            0 => if self.name[0] == 0xE5 { 0x05 } else { self.name[0] },
            b @ 1..=7 => self.name[b], 
            b @ 7..=10 => self.ext[b - 8],
            _ => 0,
        }
    }

    pub fn case_flag(self) -> u8 {
        match (self.lower_name, self.lower_ext) {
            (true, true) => 0x18, 
            (true, false) => 0x08,
            (false, true) => 0x10,
            (false, false) => 0x0,
        }
    }

    pub fn to_string(self) -> String {
        let mut retval = String::new();
        for name_idx in 0..8 {
            if self.name[name_idx] == b' ' || self.name[name_idx] == 0 {
                break;
            }
            let namechar : char = self.name[name_idx].into();
            if self.lower_name {
                retval.push(namechar.to_ascii_lowercase());
            }
            else {
                retval.push(namechar.to_ascii_uppercase());
            }
        }
        retval.push('.');
        for ext_idx in 0..8 {
            if self.ext[ext_idx] == b' ' || self.ext[ext_idx] == 0 {
                break;
            }
            let extchar : char = self.ext[ext_idx].into();
            if self.lower_ext {
                retval.push(extchar.to_ascii_lowercase());
            }
            else {
                retval.push(extchar.to_ascii_uppercase());
            }
        }


        retval 
    }

    pub fn from_str<T : AsRef<str>>(name : T) -> Option<ShortName> {
        let name : &str= name.as_ref();
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
            }
            else if is_end_marker(c) {
                ext_idx = idx; 
                break;
            }
            else if name_case == 0 && case != 0 {
                name_case = case;
                retval.lower_name = case == 1;
            }

            retval.name[idx] = char_to_byte(c);
        }
        if ext_idx == 0 {
            return None;
        }
        else if ext_idx == name.len() {
            return Some(retval);
        }
        let mut ext_case = 0;
        for (idx, c) in name.char_indices().skip(ext_idx) {
            let case = case_val(c);
            if idx > 2 || !is_valid_char(c) || name_case + case == 3 {
                return None;
            }
            else if is_end_marker(c) {
                break;
            }
            else if ext_case == 0 && case != 0 {
                ext_case = case;
                retval.lower_ext = case == 1;
            }

            retval.ext[idx] = char_to_byte(c);
        }
        Some(retval)
    }

    pub fn convert_str<T : AsRef<str>>(name : T, duplicate_count : u8) -> ShortName {
        let name : &str = name.as_ref();
        if let Some(r) = ShortName::from_str(name) {
            return r;
        }
        let ext_idx = name.char_indices().rfind(|(_, c)| *c == '.').map(|(idx, _)| idx);
        let (name_part_raw, ext_part_raw) = ext_idx.map_or( (name, ""), |idx| name.split_at(idx));
        let name_part = to_valid_shortname(name_part_raw);
        let ext_part = to_valid_shortname(ext_part_raw);

        let mut retval = ShortName::default();
        (&mut retval.ext).copy_from_slice(&ext_part.as_bytes()[0..3]);
        (&mut retval.name).copy_from_slice(&name_part.as_bytes()[0..3]);
        if duplicate_count == 0 {
            retval.name[6] = b'~';
            retval.name[7] = b'~';
        }
        else {
            let mut suffix_digits_left = duplicate_count;
            let mut cur_idx = 7;
            while suffix_digits_left > 0 {
                let digit = suffix_digits_left % 10;
                let digit_char = digit + b'0';
                retval.name[cur_idx] = digit_char;
                cur_idx += 1;
                suffix_digits_left /= 10;
            }
            retval.name[cur_idx] = b'~';
        }
        retval
    }

    pub fn lfn_checksum(&self) -> u8 {
        let mut retval : u8 = 0;
        for c in self.name.iter() {
            let shifted_retval = ((retval & 1) << 7) + ((retval & 0xFE) >> 1);
            retval = shifted_retval.wrapping_add(*c);
        }

        retval
    }

}

fn char_to_byte(assumed_valid : char) -> u8 {
    let mut tmpbuff = [0 ; 1];
    assumed_valid.encode_utf8(&mut tmpbuff);
    tmpbuff[0]
}
fn is_valid_char(inp : char) -> bool {
    inp.len_utf8() == 1 && 
    (inp.is_ascii_uppercase() || inp.is_ascii_lowercase() || inp.is_ascii_digit()
    || inp == '!' || inp == '@' || inp == '#' || inp == '$'
    || inp == '%' || inp == '^' || inp == '&' 
    || inp == '(' || inp == ')' || inp == '{' || inp == '}')
}

fn is_end_marker(inp : char) -> bool {
    inp == ' ' || inp == '.' || inp == '\0'
}

fn case_val(inp : char) -> u8 {
    if inp.is_ascii_lowercase() { 1 } else if inp.is_ascii_uppercase() { 2 } else { 0 }
}

fn to_valid_shortname<T : AsRef<str>>(raw : T) -> String {
    let raw : &str = raw.as_ref();
    raw.chars().filter_map(|c| {
        if is_end_marker(c) {
            None
        }
        else if !is_valid_char(c) {
            Some('_')
        }
        else {
            Some(c.to_ascii_uppercase())
        }
    }).collect()
}