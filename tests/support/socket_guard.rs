use std::net::TcpListener;
use std::panic::Location;

use wiremock::MockServer;

#[must_use]
pub fn socket_tests_required() -> bool {
    std::env::var("DOWNLOADER_REQUIRE_SOCKET_TESTS")
        .ok()
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

#[track_caller]
#[must_use]
pub fn should_skip_socket_bound_test() -> bool {
    if TcpListener::bind("127.0.0.1:0").is_ok() {
        return false;
    }

    let location = Location::caller();
    let message = format!(
        "[socket-bound-test] cannot bind localhost socket at {}:{}; wiremock-based test cannot run in this environment",
        location.file(),
        location.line()
    );
    if socket_tests_required() {
        panic!("{message}. Set DOWNLOADER_REQUIRE_SOCKET_TESTS=0 to allow local skip behavior.");
    }

    eprintln!(
        "{message}. Skipping test. Set DOWNLOADER_REQUIRE_SOCKET_TESTS=1 to fail-fast instead."
    );
    true
}

pub async fn start_mock_server_or_skip() -> Option<MockServer> {
    if should_skip_socket_bound_test() {
        None
    } else {
        Some(MockServer::start().await)
    }
}

#[allow(dead_code)]
pub trait SocketSkipReturn {
    fn socket_skip_return() -> Self;
}

impl SocketSkipReturn for () {
    fn socket_skip_return() -> Self {}
}

impl SocketSkipReturn for Result<(), Box<dyn std::error::Error>> {
    fn socket_skip_return() -> Self {
        Ok(())
    }
}

#[allow(dead_code)]
pub fn socket_skip_return<T: SocketSkipReturn>() -> T {
    T::socket_skip_return()
}
