#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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

impl ShortName {
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
}