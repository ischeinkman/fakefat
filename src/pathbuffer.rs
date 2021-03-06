#[cfg(feature = "alloc")]
pub use with_alloc::PathBuff;
#[cfg(feature = "alloc")]
mod with_alloc {

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    extern crate alloc;

    #[cfg(feature = "std")]
    use std as alloc;

    use alloc::vec::Vec;
    use core::str::from_utf8_unchecked;

    use core::fmt;

    #[derive(Hash, Clone)]
    pub struct PathBuff {
        bytes: Vec<u8>,
        is_file: bool,
    }
    impl PathBuff {
        pub fn add_subdir(&mut self, component: &str) {
            debug_assert!(!self.is_file);
            self.bytes.extend_from_slice(component.as_bytes());
            if !self.bytes.ends_with(&[b'/']) {
                self.bytes.push(b'/');
            }
        }

        pub fn add_file(&mut self, file_name: &str) {
            debug_assert!(!self.is_file);
            self.bytes.extend_from_slice(file_name.as_bytes());
            self.is_file = true;
        }

        pub fn to_str(&self) -> &str {
            unsafe { from_utf8_unchecked(self.bytes.as_slice()) }
        }
    }

    impl fmt::Display for PathBuff {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.to_str())
        }
    }

    impl Default for PathBuff {
        fn default() -> Self {
            PathBuff {
                bytes: Vec::from([b'/'].as_ref()),
                is_file: false,
            }
        }
    }

}

#[cfg(not(feature = "alloc"))]
pub use fixed_size::PathBuff;
#[cfg(not(feature = "alloc"))]
mod fixed_size {
    use core;
    use core::fmt;
    use core::str::from_utf8_unchecked;
    mod sizes {
        pub const ELEMENTS: usize = 128;
    }

    use sizes::ELEMENTS;
    
    #[derive(Clone)]
    pub struct PathBuff {
        data: [u8; ELEMENTS],
        len: usize,
        is_file: bool,
    }

    use core::hash::{Hash, Hasher};
    impl Hash for PathBuff {
        fn hash<H : Hasher>(&self, hasher : &mut H) {
            self.to_str().hash(hasher);
        }
    }

    impl PathBuff {
        pub fn add_subdir(&mut self, component: &str) {
            debug_assert!(!self.is_file);
            let comp_bytes = component.as_bytes();
            debug_assert!(ELEMENTS - self.len >= comp_bytes.len());
            let data_slice = &mut self.data[self.len .. self.len + comp_bytes.len()];
            data_slice.copy_from_slice(comp_bytes);
            if !self.data[self.len + comp_bytes.len() - 1] == b'/' {
                self.data[self.len + comp_bytes.len()] = b'/';
            }
            self.len += comp_bytes.len() + 1;
        }

        pub fn add_file(&mut self, file_name: &str) {
            debug_assert!(!self.is_file);
            let comp_bytes = file_name.as_bytes();
            debug_assert!(ELEMENTS - self.len >= comp_bytes.len());
            let data_slice = &mut self.data[self.len .. self.len + comp_bytes.len()];
            data_slice.copy_from_slice(comp_bytes);
            self.len += comp_bytes.len();
            self.is_file = true;
        }
        pub fn to_str(&self) -> &str {
            unsafe { from_utf8_unchecked(&self.data[0..self.len]) }
        }
    }

    impl fmt::Display for PathBuff {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.to_str())
        }
    }

    impl Default for PathBuff {
        fn default() -> Self {
            let mut retval = PathBuff {
                data: [0; ELEMENTS],
                len: 0,
                is_file: false,
            };
            retval.data[0] = b'/';
            retval.len = 1;
            retval
        }
    }
}
