use crate::ffi::OsStr;
use crate::path::Prefix;

#[inline]
pub fn is_sep_byte(b: u8) -> bool {
    b == b'/'
}

#[inline]
pub fn is_verbatim_sep(b: u8) -> bool {
    b == b'/'
}

pub fn parse_prefix(path: &OsStr) -> Option<Prefix<'_>> {
    if let Some(path_str) = path.to_str() {
        if let Some(i) = path_str.find(':') {
            Some(Prefix::Scheme(OsStr::new(&path_str[..i])))
        } else {
            None
        }
    } else {
        None
    }
}

pub const MAIN_SEP_STR: &str = "/";
pub const MAIN_SEP: char = '/';
