//! Phase 3 (P0): Malformed cookie data, XSS vectors.
//! Parse/store/load either reject or sanitize.

use std::io::Cursor;

use downloader_core::parse_netscape_cookies;

#[test]
fn p0_malformed_netscape_line_warning_not_crash() {
    let input = ".example.com\tTRUE\t/\tFALSE\t0\tname\tvalue\n\
                 bad\tfields\n\
                 .other.com\tTRUE\t/\tFALSE\t0\to\tv\n";
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader).expect("parse returns result");
    assert_eq!(result.cookies.len(), 2);
    assert!(!result.warnings.is_empty());
}

#[test]
fn p0_all_malformed_returns_error() {
    let input = "not a cookie line\n<script>alert(1)</script>\n";
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader);
    assert!(result.is_err(), "all malformed should error");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("cookie") || msg.to_lowercase().contains("valid"),
        "error should mention cookies: {}",
        msg
    );
}

#[test]
fn p0_cookie_value_with_special_chars_parsed() {
    let input = ".example.com	TRUE	/	FALSE	0	name	value-with-special=chars\n";
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader).expect("parse");
    assert_eq!(result.cookies.len(), 1);
    assert_eq!(result.cookies[0].value(), "value-with-special=chars");
}
