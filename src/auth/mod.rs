//! Authentication and cookie management.
//!
//! This module provides cookie loading from Netscape-format cookie files,
//! which can be exported from browsers or browser extensions.

mod capture;
mod cookies;
mod runtime_cookies;
mod storage;

pub use capture::{
    CaptureError, CapturedCookieFormat, CapturedCookies, parse_captured_cookies,
    unique_domain_count,
};
pub use cookies::{
    CookieError, CookieLine, ParseResult, load_cookies_into_jar, parse_netscape_cookies,
};
pub use runtime_cookies::load_runtime_cookie_jar;
pub use storage::{
    StorageError, clear_persisted_cookies, load_persisted_cookies, persisted_cookie_path,
    rotate_key, store_persisted_cookies,
};
