//! Integration tests for the download module.
//!
//! These tests verify the full download flow with mock HTTP servers.

use std::path::Path;

use downloader_core::download::{DownloadError, HttpClient};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a mock server with a file endpoint.
async fn setup_mock_file(path_str: &str, content: &[u8]) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(path_str))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(content.to_vec()))
        .mount(&mock_server)
        .await;

    mock_server
}

#[tokio::test]
async fn test_download_full_flow_preserves_content() {
    // Setup
    let content = b"This is the complete file content for testing.\nLine 2.\nLine 3.";
    let mock_server = setup_mock_file("/document.pdf", content).await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Execute
    let client = HttpClient::new();
    let url = format!("{}/document.pdf", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    // Verify
    assert!(
        result.is_ok(),
        "Download should succeed: {:?}",
        result.err()
    );

    let file_path = result.unwrap();
    assert!(file_path.exists(), "Downloaded file should exist");

    let downloaded_content = std::fs::read(&file_path).expect("should read file");
    assert_eq!(
        downloaded_content, content,
        "Downloaded content should match original"
    );
}

#[tokio::test]
async fn test_download_uses_content_disposition_filename() {
    let mock_server = MockServer::start().await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    Mock::given(method("GET"))
        .and(path("/api/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Content-Disposition",
                    r#"attachment; filename="important-paper.pdf""#,
                )
                .set_body_bytes(b"PDF bytes"),
        )
        .mount(&mock_server)
        .await;

    let client = HttpClient::new();
    let url = format!("{}/api/download", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_ok());
    let file_path = result.unwrap();
    assert_eq!(
        file_path.file_name().unwrap().to_str().unwrap(),
        "important-paper.pdf"
    );
}

#[tokio::test]
async fn test_download_extracts_filename_from_url() {
    let mock_server = setup_mock_file("/papers/research-2024.pdf", b"content").await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    let client = HttpClient::new();
    let url = format!("{}/papers/research-2024.pdf", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_ok());
    let file_path = result.unwrap();
    assert!(
        file_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("research-2024"),
        "Filename should contain 'research-2024': {:?}",
        file_path
    );
}

#[tokio::test]
async fn test_download_handles_404_gracefully() {
    let mock_server = MockServer::start().await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    Mock::given(method("GET"))
        .and(path("/not-found"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = HttpClient::new();
    let url = format!("{}/not-found", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_err());
    match result {
        Err(DownloadError::HttpStatus {
            status,
            url: err_url,
            ..
        }) => {
            assert_eq!(status, 404);
            assert!(err_url.contains("/not-found"));
        }
        other => panic!("Expected HttpStatus(404), got: {:?}", other),
    }
}

#[tokio::test]
async fn test_download_handles_500_error() {
    let mock_server = MockServer::start().await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    Mock::given(method("GET"))
        .and(path("/server-error"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = HttpClient::new();
    let url = format!("{}/server-error", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_err());
    match result {
        Err(DownloadError::HttpStatus { status, .. }) => {
            assert_eq!(status, 500);
        }
        other => panic!("Expected HttpStatus(500), got: {:?}", other),
    }
}

#[tokio::test]
async fn test_download_handles_duplicate_filenames() {
    let mock_server = setup_mock_file("/doc.pdf", b"content").await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Create existing file
    std::fs::write(temp_dir.path().join("doc.pdf"), b"existing").expect("should create file");

    let client = HttpClient::new();
    let url = format!("{}/doc.pdf", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_ok());
    let file_path = result.unwrap();

    // Should be doc_1.pdf, not doc.pdf
    let filename = file_path.file_name().unwrap().to_str().unwrap();
    assert!(
        filename.contains("_1") || filename.contains("_2"),
        "Filename should have numeric suffix: {}",
        filename
    );
}

#[tokio::test]
async fn test_download_rejects_invalid_url() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let client = HttpClient::new();

    let result = client
        .download_to_file("definitely-not-a-url", temp_dir.path())
        .await;

    assert!(result.is_err());
    assert!(
        matches!(result, Err(DownloadError::InvalidUrl { .. })),
        "Expected InvalidUrl, got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_download_client_is_reusable() {
    let mock_server = MockServer::start().await;
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    Mock::given(method("GET"))
        .and(path("/file1.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"file1"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/file2.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"file2"))
        .mount(&mock_server)
        .await;

    let client = HttpClient::new();

    // Download first file
    let url1 = format!("{}/file1.txt", mock_server.uri());
    let result1 = client.download_to_file(&url1, temp_dir.path()).await;
    assert!(result1.is_ok());

    // Reuse same client for second download
    let url2 = format!("{}/file2.txt", mock_server.uri());
    let result2 = client.download_to_file(&url2, temp_dir.path()).await;
    assert!(result2.is_ok());

    // Verify both files exist with correct content
    let path1 = result1.unwrap();
    let path2 = result2.unwrap();

    assert_eq!(std::fs::read(&path1).unwrap(), b"file1");
    assert_eq!(std::fs::read(&path2).unwrap(), b"file2");
}

#[tokio::test]
async fn test_download_to_nonexistent_directory_fails() {
    let mock_server = setup_mock_file("/file.txt", b"content").await;
    let nonexistent = Path::new("/this/path/definitely/does/not/exist/anywhere");

    let client = HttpClient::new();
    let url = format!("{}/file.txt", mock_server.uri());
    let result = client.download_to_file(&url, nonexistent).await;

    assert!(result.is_err());
    assert!(
        matches!(result, Err(DownloadError::Io { .. })),
        "Expected IO error, got: {:?}",
        result
    );
}
