//! Netascii string utilities.
use std::borrow::{Cow, IntoCow};

/// Netascii encoded string
pub type NetasciiString<'a> = Cow<'a, str>;

fn is_escape_required(s: &str) -> bool {
    s.chars().any(|c| c == '\r' || c == '\n')
}

/// Converts a netascii encoded string into utf-8 unescaped string without performaing
/// any allocations if possible.
///
/// If the input does not contain any escaped characters '\r' or '\n', input string slice
/// is returned as is.
/// If input contains escaped characters new string is allocated, unescaped and
/// returned.
///
/// Returns `None` if the input string contains invalid scape sequence.
pub fn from_netascii<'a>(s: &'a str) -> Option<Cow<'a, str>> {
    if !is_escape_required(s) {
        return Some(s.into_cow())
    }
    let mut decoded = String::new();
    let mut chars = s.chars();
    loop {
        let next = chars.next();
        match next {
            Some('\r') => {
                match chars.next() {
                    Some('\n') => decoded.push('\n'),
                    Some('\0') => decoded.push('\r'),
                    _ => return None
                }
            }
            Some(c) => decoded.push(c),
            None => break
        }
    }
    return Some(decoded.into_cow())
}

/// Coverts a string slice into netascii encoded string without performing any
/// allocations if possible.
///
/// If the input does not cantain any of '\r' or '\n' characters, input string
/// slice is returned as is.
/// If escaping is required new string is allocated, escaped and returned.
pub fn to_netascii<'a>(s: &'a str) -> NetasciiString<'a> {
    if !is_escape_required(s) {
        return s.into_cow()
    }
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            '\n' => encoded.push_str("\r\n"),
            '\r' => encoded.push_str("\r\0"),
            _ => encoded.push(c)
        }
    }
    return encoded.into_cow()
}

#[cfg(test)]
mod test {
    use std::borrow::IntoCow;

    use super::{from_netascii, to_netascii};

    static TEXT_NORMAL: &'static str = "\tfoo\nbar\r\nbaz";
    static TEXT_NETASCII: &'static str = "\tfoo\r\nbar\r\0\r\nbaz";

    static TEXT_NOESCAPE: &'static str = "foo\tbar\0baz";

    #[test]
    fn from_netascii_newline_is_unescaped() {
        let decoded = from_netascii("\r\n");
        assert_eq!(Some("\n".into_cow()), decoded);
    }

    #[test]
    fn from_netascii_carriage_return_is_unescaped() {
        let decoded = from_netascii("\r\0");
        assert_eq!(Some("\r".into_cow()), decoded);
    }

    #[test]
    fn from_netascii_string_with_escaping() {
        let decoded = from_netascii(TEXT_NETASCII);
        assert_eq!(Some(TEXT_NORMAL.into_cow()), decoded);
    }

    #[test]
    fn from_netascii_string_without_escaping() {
        let decoded = from_netascii(TEXT_NOESCAPE);
        assert_eq!(Some(TEXT_NOESCAPE.into_cow()), decoded);
    }

    #[test]
    fn to_netascii_newline_is_escaped() {
        let decoded = to_netascii("\n");
        assert_eq!("\r\n".into_cow(), decoded);
    }

    #[test]
    fn to_netascii_carriage_return_is_escaped() {
        let decoded = to_netascii("\r");
        assert_eq!("\r\0".into_cow(), decoded);
    }

    #[test]
    fn to_netascii_string_with_escaping() {
        let decoded = to_netascii(TEXT_NORMAL);
        assert_eq!(TEXT_NETASCII.into_cow(), decoded);
    }

    #[test]
    fn to_netascii_string_without_escaping() {
        let decoded = to_netascii(TEXT_NOESCAPE);
        assert_eq!(TEXT_NOESCAPE.into_cow(), decoded);
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use self::test::{Bencher, black_box};

    use super::{from_netascii, to_netascii};

    static TEXT_DATA: &'static str = include_str!("../../data/lipsum.txt");

    #[bench]
    fn from_netascii_with_encoding(b: &mut Bencher) {
        let netascii = to_netascii(TEXT_DATA);
        b.iter(|| {
            black_box(from_netascii(netascii.as_slice()));
        });
        b.bytes = TEXT_DATA.as_bytes().len() as u64;
    }

    #[bench]
    fn from_netascii_without_encoding(b: &mut Bencher) {
        let no_newlines = TEXT_DATA.replace("\n", "");
        b.iter(|| {
            black_box(from_netascii(no_newlines.as_slice()));
        });
        b.bytes = TEXT_DATA.as_bytes().len() as u64;
    }

    #[bench]
    fn to_netascii_with_encoding(b: &mut Bencher) {
        b.iter(|| {
            black_box(to_netascii(TEXT_DATA));
        });
        b.bytes = TEXT_DATA.as_bytes().len() as u64;
    }

    #[bench]
    fn to_netascii_without_encoding(b: &mut Bencher) {
        let no_newlines = TEXT_DATA.replace("\n", "");
        b.iter(|| {
            black_box(to_netascii(no_newlines.as_slice()));
        });
        b.bytes = TEXT_DATA.as_bytes().len() as u64;
    }
}
