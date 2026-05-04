//! pfp-tile:// async URI scheme handler. RED-gate stub: the parse_zxy unit
//! tests below MUST fail against this body. Task 2 GREEN replaces this with
//! the full implementation.

#![allow(dead_code)]

use tauri::{UriSchemeContext, UriSchemeResponder};

pub(crate) fn parse_zxy(_path: &str) -> Option<(u8, u32, u32)> {
    // RED: deliberate wrong answer so unit tests fail.
    None
}

pub fn handle<R: tauri::Runtime>(
    _ctx: UriSchemeContext<'_, R>,
    _request: http::Request<Vec<u8>>,
    _responder: UriSchemeResponder,
) {
    unimplemented!("Task 2 GREEN replaces this");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zxy_happy_path() {
        assert_eq!(parse_zxy("/14/13708/6334.png"), Some((14, 13708, 6334)));
    }

    #[test]
    fn parse_zxy_zoom_zero_ok() {
        assert_eq!(parse_zxy("/0/0/0.png"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_zxy_zoom_nineteen_ok() {
        assert_eq!(parse_zxy("/19/100/100.png"), Some((19, 100, 100)));
    }

    #[test]
    fn parse_zxy_rejects_zoom_above_nineteen() {
        assert!(parse_zxy("/20/0/0.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_missing_png_suffix() {
        assert!(parse_zxy("/14/13708/6334").is_none());
    }

    #[test]
    fn parse_zxy_rejects_missing_leading_slash() {
        assert!(parse_zxy("14/13708/6334.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_non_numeric_z() {
        assert!(parse_zxy("/abc/13708/6334.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_extra_segment() {
        assert!(parse_zxy("/14/13708/6334.png/extra").is_none());
    }

    #[test]
    fn parse_zxy_rejects_path_traversal() {
        assert!(parse_zxy("/../1/1.png").is_none());
        assert!(parse_zxy("/14/../1.png").is_none());
    }
}
