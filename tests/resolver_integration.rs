//! Integration tests for the resolver module.
//!
//! Tests the full resolution flow through the public API.

use std::sync::Arc;

use downloader_core::parser::InputType;
use downloader_core::resolver::{
    ArxivResolver, CrossrefResolver, DirectResolver, IeeeResolver, PubMedResolver, ResolveContext,
    ResolvedUrl, ResolverRegistry, STANDARD_METADATA_KEYS, ScienceDirectResolver, SpringerResolver,
    build_default_resolver_registry,
};
use reqwest::cookie::Jar;
use wiremock::matchers::{header_regex, method, path, path_regex};
use wiremock::{Mock, ResponseTemplate};

mod support;
use support::socket_guard::start_mock_server_or_skip;

#[tokio::test]
async fn test_resolver_registry_with_direct_resolver() {
    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("https://example.com/paper.pdf", InputType::Url, &ctx)
        .await;

    assert!(result.is_ok(), "DirectResolver should resolve URLs");
    let resolved = result.unwrap();
    assert_eq!(resolved.url, "https://example.com/paper.pdf");
}

#[tokio::test]
async fn test_resolver_registry_rejects_doi_with_only_direct() {
    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("10.1234/test", InputType::Doi, &ctx)
        .await;

    assert!(result.is_err(), "DirectResolver should not handle DOIs");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("no resolver"),
        "Expected 'no resolver' error, got: {err}"
    );
}

#[tokio::test]
async fn test_resolver_direct_preserves_url_exactly() {
    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let urls = [
        "https://example.com/paper.pdf",
        "https://arxiv.org/pdf/2301.00001.pdf",
        "http://example.com/path?query=value&other=123#fragment",
    ];

    for url in urls {
        let result = registry.resolve_to_url(url, InputType::Url, &ctx).await;
        assert!(result.is_ok(), "Should resolve: {url}");
        assert_eq!(result.unwrap().url, url, "URL should be preserved exactly");
    }
}

#[tokio::test]
async fn test_resolver_registry_multiple_resolvers_priority() {
    use async_trait::async_trait;
    use downloader_core::resolver::{
        ResolveContext, ResolveError, ResolveStep, Resolver, ResolverPriority,
    };

    /// A mock resolver that returns a specific URL, used to verify priority ordering.
    struct PriorityMockResolver {
        resolver_name: &'static str,
        resolver_priority: ResolverPriority,
        result_url: &'static str,
    }

    #[async_trait]
    impl Resolver for PriorityMockResolver {
        fn name(&self) -> &str {
            self.resolver_name
        }

        fn priority(&self) -> ResolverPriority {
            self.resolver_priority
        }

        fn can_handle(&self, _input: &str, input_type: InputType) -> bool {
            input_type == InputType::Url
        }

        async fn resolve(
            &self,
            _input: &str,
            _ctx: &ResolveContext,
        ) -> Result<ResolveStep, ResolveError> {
            Ok(ResolveStep::Url(ResolvedUrl::new(self.result_url)))
        }
    }

    let mut registry = ResolverRegistry::new();

    // Register in reverse priority order - Fallback first, Specialized last
    registry.register(Box::new(PriorityMockResolver {
        resolver_name: "fallback",
        resolver_priority: ResolverPriority::Fallback,
        result_url: "https://fallback.com",
    }));
    registry.register(Box::new(PriorityMockResolver {
        resolver_name: "specialized",
        resolver_priority: ResolverPriority::Specialized,
        result_url: "https://specialized.com",
    }));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("https://example.com", InputType::Url, &ctx)
        .await;

    assert!(result.is_ok());
    // Specialized should win over Fallback regardless of registration order
    assert_eq!(
        result.unwrap().url,
        "https://specialized.com",
        "Specialized resolver should take priority over Fallback"
    );
}

// ==================== Crossref Resolver Integration Tests ====================

fn crossref_success_json() -> serde_json::Value {
    serde_json::json!({
        "status": "ok",
        "message": {
            "title": ["Integration Test Paper"],
            "author": [{"given": "Alice", "family": "Researcher"}],
            "link": [{
                "URL": "https://publisher.com/fulltext.pdf",
                "content-type": "application/pdf",
                "content-version": "vor",
                "intended-application": "text-mining"
            }],
            "published": {"date-parts": [[2024, 3]]}
        }
    })
}

fn crossref_no_pdf_json() -> serde_json::Value {
    serde_json::json!({
        "status": "ok",
        "message": {
            "title": ["No PDF Paper"],
            "published": {"date-parts": [[2023]]}
        }
    })
}

#[tokio::test]
async fn test_crossref_resolver_in_registry_resolves_doi_to_pdf() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path_regex(r"/works/10\..+"))
        .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("10.1234/test.paper", InputType::Doi, &ctx)
        .await;

    assert!(result.is_ok(), "Should resolve DOI via Crossref");
    let resolved = result.unwrap();
    assert_eq!(resolved.url, "https://publisher.com/fulltext.pdf");
    assert_eq!(
        resolved.metadata.get("title").unwrap(),
        "Integration Test Paper"
    );
    assert_eq!(
        resolved.metadata.get("authors").unwrap(),
        "Researcher, Alice"
    );
}

#[tokio::test]
async fn test_crossref_resolver_no_pdf_redirects_through_direct() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path_regex(r"/works/10\..+"))
        .respond_with(ResponseTemplate::new(200).set_body_json(crossref_no_pdf_json()))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("10.5678/no-pdf", InputType::Doi, &ctx)
        .await;

    // Crossref returns Redirect(doi.org URL), registry follows redirect,
    // DirectResolver handles the doi.org URL
    assert!(result.is_ok(), "Should resolve via redirect fallback");
    let resolved = result.unwrap();
    assert_eq!(resolved.url, "https://doi.org/10.5678/no-pdf");
}

#[tokio::test]
async fn test_crossref_resolver_404_fails_gracefully() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path_regex(r"/works/10\..+"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap(),
    ));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("10.9999/invalid", InputType::Doi, &ctx)
        .await;

    assert!(result.is_err(), "Should fail for unknown DOI");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("all resolvers failed"),
        "Error should mention 'all resolvers failed': {err}"
    );
}

#[tokio::test]
async fn test_sciencedirect_resolver_url_with_cookies_resolves_pdf_and_metadata() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let pii = "S0167739X18313560";
    let article_path = format!("/science/article/pii/{pii}");
    let pdf_path = format!("/science/article/pii/{pii}/pdfft?isDTMRedir=true&download=true");

    let html = format!(
        r#"
        <html>
          <head>
            <meta name="citation_title" content="ScienceDirect Integration Test">
            <meta name="citation_author" content="Alice Researcher">
            <meta name="citation_author" content="Bob Scientist">
            <meta name="citation_doi" content="10.1016/j.future.2018.10.001">
            <meta name="citation_pdf_url" content="{pdf_path}">
          </head>
        </html>
        "#
    );

    Mock::given(method("GET"))
        .and(path_regex(r"/science/article/pii/S0167739X18313560$"))
        .and(header_regex("cookie", "SDSESSION=valid-session"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(&mock_server)
        .await;

    let jar = Arc::new(Jar::default());
    let origin = url::Url::parse(&mock_server.uri()).unwrap();
    jar.add_cookie_str("SDSESSION=valid-session; Path=/", &origin);

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(Some(jar), mock_server.uri(), mock_server.uri())
            .unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let article_url = format!("{}{article_path}", mock_server.uri());
    let result = registry
        .resolve_to_url(&article_url, InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, format!("{}{pdf_path}", mock_server.uri()));
    assert_eq!(
        result.metadata.get("title").unwrap(),
        "ScienceDirect Integration Test"
    );
    assert_eq!(
        result.metadata.get("authors").unwrap(),
        "Alice Researcher; Bob Scientist"
    );
    assert_eq!(
        result.metadata.get("doi").unwrap(),
        "10.1016/j.future.2018.10.001"
    );
}

#[tokio::test]
async fn test_sciencedirect_resolver_can_handle_elsevier_doi() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    let doi = "10.1016/j.future.2018.10.001";
    let article_path = "/science/article/abs/pii/S0167739X18313560";
    let pdf_url = format!(
        "{}/science/article/pii/S0167739X18313560/pdfft?isDTMRedir=true&download=true",
        mock_server.uri()
    );

    Mock::given(method("GET"))
        .and(path_regex(r"/10\.1016/.+"))
        .respond_with(
            ResponseTemplate::new(302)
                .append_header("location", format!("{}{}", mock_server.uri(), article_path)),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/science/article/abs/pii/S0167739X18313560$"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<meta name="citation_title" content="DOI to ScienceDirect">
               <meta name="citation_pdf_url" content="{pdf_url}">
               <meta name="citation_doi" content="{doi}">"#
        )))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(
        CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url(doi, InputType::Doi, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, pdf_url);
    assert_eq!(result.metadata.get("doi").unwrap(), doi);
}

#[tokio::test]
async fn test_sciencedirect_auth_page_suggests_cookie_refresh() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let article_url = format!(
        "{}/science/article/pii/S0167739X18313560",
        mock_server.uri()
    );

    Mock::given(method("GET"))
        .and(path_regex(r"/science/article/pii/S0167739X18313560$"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><body>id.elsevier.com Sign in with your institution</body></html>",
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url(&article_url, InputType::Url, &ctx)
        .await;

    assert!(result.is_err(), "Expected auth-required style error");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("authentication required"));
    assert!(err.to_ascii_lowercase().contains("refresh cookies"));
}

#[tokio::test]
async fn test_sciencedirect_direct_pdf_url_bypasses_page_resolution() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let base_url = mock_server.uri();
    let pdf_url = format!(
        "{}/science/article/pii/S0167739X18313560/pdfft?isDTMRedir=true&download=true",
        base_url
    );

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(None, &base_url, &base_url).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url(&pdf_url, InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, pdf_url);
    assert!(result.metadata.is_empty());

    let requests = mock_server.received_requests().await.unwrap();
    assert!(
        requests.is_empty(),
        "direct PDF URLs should not trigger ScienceDirect page fetches"
    );
}

// ==================== Regression Tests for Code Review Issues ====================

#[tokio::test]
async fn regression_sciencedirect_sends_user_agent_header() {
    // H2: Verify User-Agent header is sent to avoid bot detection
    // ScienceDirect may block requests without proper User-Agent
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let article_path = "/science/article/pii/S0167739X18313560";

    Mock::given(method("GET"))
        .and(path_regex(r"/science/article/pii/S0167739X18313560$"))
        .and(header_regex(
            "user-agent",
            r"downloader/[0-9.]+ \(research-tool; \+https://github\.com/nicksrandall/Downloader\)",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<meta name="citation_title" content="Test">
               <meta name="citation_pdf_url" content="/test.pdf">"#,
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let article_url = format!("{}{}", mock_server.uri(), article_path);
    let result = registry
        .resolve_to_url(&article_url, InputType::Url, &ctx)
        .await;

    assert!(
        result.is_ok(),
        "Request should succeed when User-Agent is properly set"
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "Exactly one request should be made");

    let user_agent = requests[0]
        .headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok());
    assert!(
        user_agent.is_some(),
        "User-Agent header must be present in request"
    );
    assert!(
        user_agent.unwrap().contains("downloader"),
        "User-Agent should identify as downloader: {}",
        user_agent.unwrap()
    );
    assert!(
        user_agent.unwrap().contains("research-tool"),
        "User-Agent should use shared resolver format (research-tool): {}",
        user_agent.unwrap()
    );
    assert!(
        !user_agent.unwrap().contains("sciencedirect"),
        "User-Agent must not contain resolver name (shared UA): {}",
        user_agent.unwrap()
    );
}

/// Regression: Springer (and all resolvers) must send the shared UA format, not per-resolver UA.
#[tokio::test]
async fn regression_springer_sends_shared_user_agent() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let article_path = "/article/10.1007/s00134-020-06294-x";
    let pdf_path = "/content/pdf/10.1007/s00134-020-06294-x.pdf";

    Mock::given(method("GET"))
        .and(path(article_path))
        .and(header_regex(
            "user-agent",
            r"downloader/[0-9.]+ \(research-tool; \+https://github\.com/nicksrandall/Downloader\)",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<meta name="citation_title" content="Test">
               <meta name="citation_doi" content="10.1007/s00134-020-06294-x">
               <meta name="citation_publication_date" content="2023-04-01">
               <meta name="citation_pdf_url" content="{pdf_path}">"#
        )))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        SpringerResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let input_url = format!("{}{}", mock_server.uri(), article_path);
    let result = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await;

    assert!(
        result.is_ok(),
        "Request should succeed when User-Agent is shared format: {:?}",
        result.err()
    );

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "Exactly one request should be made");
    let ua = requests[0]
        .headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    assert!(
        ua.contains("downloader"),
        "UA must identify as downloader: {ua}"
    );
    assert!(
        ua.contains("research-tool"),
        "UA must use shared resolver format: {ua}"
    );
    assert!(
        !ua.contains("springer"),
        "UA must not contain resolver name (shared UA): {ua}"
    );
}

#[tokio::test]
async fn regression_sciencedirect_resolver_construction_handles_errors() {
    // M1: Verify resolver construction returns Result and handles errors gracefully
    // Library code must not panic on construction
    let result = ScienceDirectResolver::new(None);
    assert!(
        result.is_ok(),
        "ScienceDirectResolver::new should return Ok(resolver)"
    );

    let jar = Arc::new(Jar::default());
    let result_with_jar = ScienceDirectResolver::new(Some(jar.clone()));
    assert!(
        result_with_jar.is_ok(),
        "ScienceDirectResolver::new with cookie jar should return Ok(resolver)"
    );

    let result_with_urls = ScienceDirectResolver::with_base_urls(
        Some(jar),
        "https://www.sciencedirect.com",
        "https://doi.org",
    );
    assert!(
        result_with_urls.is_ok(),
        "ScienceDirectResolver::with_base_urls should return Ok(resolver)"
    );
}

#[tokio::test]
async fn regression_sciencedirect_linkinghub_doi_redirect_accepted() {
    // M5: Verify linkinghub.elsevier.com paths are accepted during DOI resolution
    // Some Elsevier DOIs redirect through linkinghub before reaching ScienceDirect
    // This test validates the fix that added linkinghub.elsevier.com to accepted hosts
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let doi = "10.1016/j.test.2024.01.001";

    // DOI redirects to a linkinghub-style path, then to final article page
    Mock::given(method("GET"))
        .and(path_regex(r"/10\.1016/.+"))
        .respond_with(ResponseTemplate::new(302).append_header(
            "location",
            format!("{}/retrieve/pii/S0167739X18313560", mock_server.uri()),
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/retrieve/pii/S0167739X18313560$"))
        .respond_with(ResponseTemplate::new(302).append_header(
            "location",
            format!(
                "{}/science/article/pii/S0167739X18313560",
                mock_server.uri()
            ),
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"/science/article/pii/S0167739X18313560$"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<meta name="citation_title" content="Linkinghub Redirect Test">
               <meta name="citation_pdf_url" content="/test.pdf">
               <meta name="citation_doi" content="10.1016/j.test.2024.01.001">"#,
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        ScienceDirectResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry.resolve_to_url(doi, InputType::Doi, &ctx).await;

    assert!(
        result.is_ok(),
        "DOI resolution through linkinghub-style paths should succeed: {:?}",
        result.as_ref().err()
    );
    let resolved = result.unwrap();
    assert_eq!(
        resolved.metadata.get("title").unwrap(),
        "Linkinghub Redirect Test"
    );
}

// ==================== Story 8.5 Additional Resolver Coverage ====================

#[tokio::test]
async fn test_arxiv_resolver_normalizes_abs_and_doi_inputs() {
    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(ArxivResolver::new()));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();

    let abs_result = registry
        .resolve_to_url("https://arxiv.org/abs/2301.12345v2", InputType::Url, &ctx)
        .await
        .unwrap();
    assert_eq!(abs_result.url, "https://arxiv.org/pdf/2301.12345v2.pdf");

    let doi_result = registry
        .resolve_to_url("10.48550/arXiv.2301.12345", InputType::Doi, &ctx)
        .await
        .unwrap();
    assert_eq!(doi_result.url, "https://arxiv.org/pdf/2301.12345.pdf");
}

#[tokio::test]
async fn test_pubmed_resolver_resolves_pubmed_record_to_pmc_pdf() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path("/12345678/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<html><body><a href="/articles/PMC1234567/">Open in PMC</a></body></html>"#,
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/articles/PMC1234567/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<meta name="citation_pdf_url" content="/articles/PMC1234567/pdf/main.pdf">"#,
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        PubMedResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let input_url = format!("{}/12345678/", mock_server.uri());
    let result = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(
        result.url,
        format!("{}/articles/PMC1234567/pdf/main.pdf", mock_server.uri())
    );
    assert_eq!(result.metadata.get("pmcid").unwrap(), "PMC1234567");
}

#[tokio::test]
async fn test_pubmed_resolver_returns_clear_failure_when_no_full_text() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path("/12345679/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><body>No PMC link available for this entry.</body></html>"),
        )
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        PubMedResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));

    let ctx = ResolveContext::default();
    let input_url = format!("{}/12345679/", mock_server.uri());
    let err = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap_err()
        .to_string();

    assert!(err.contains("all resolvers failed"));
}

#[tokio::test]
async fn test_ieee_resolver_extracts_stamp_pdf_from_document_page() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let document_path = "/document/1234567/";
    let stamp_path = "/stamp/stamp.jsp?tp=&arnumber=1234567";

    Mock::given(method("GET"))
        .and(path(document_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<meta name="citation_title" content="IEEE Integration">
               <meta name="citation_doi" content="10.1109/5.771073">
               <meta name="citation_publication_date" content="2024-05-20">
               <meta name="citation_pdf_url" content="{stamp_path}">"#
        )))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        IeeeResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let input_url = format!("{}{}", mock_server.uri(), document_path);
    let result = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, format!("{}{}", mock_server.uri(), stamp_path));
    assert_eq!(result.metadata.get("doi").unwrap(), "10.1109/5.771073");
    assert_eq!(result.metadata.get("year").unwrap(), "2024");
}

#[tokio::test]
async fn test_ieee_resolver_surfaces_auth_required_for_paywalled_page() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let document_path = "/document/7654321/";

    Mock::given(method("GET"))
        .and(path(document_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><body>Sign in for access through your institution. Purchase PDF.</body></html>",
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        IeeeResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));

    let ctx = ResolveContext::default();
    let input_url = format!("{}{}", mock_server.uri(), document_path);
    let err = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap_err()
        .to_string();

    assert!(err.contains("authentication required"));
}

#[tokio::test]
async fn test_springer_resolver_extracts_canonical_pdf_url() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let article_path = "/article/10.1007/s00134-020-06294-x";
    let pdf_path = "/content/pdf/10.1007/s00134-020-06294-x.pdf";

    Mock::given(method("GET"))
        .and(path(article_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            r#"<meta name="citation_title" content="Springer Integration">
               <meta name="citation_doi" content="10.1007/s00134-020-06294-x">
               <meta name="citation_publication_date" content="2023-04-01">
               <meta name="citation_pdf_url" content="{pdf_path}">"#
        )))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        SpringerResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let input_url = format!("{}{}", mock_server.uri(), article_path);
    let result = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, format!("{}{}", mock_server.uri(), pdf_path));
    assert_eq!(
        result.metadata.get("doi").unwrap(),
        "10.1007/s00134-020-06294-x"
    );
    assert_eq!(result.metadata.get("year").unwrap(), "2023");
}

#[tokio::test]
async fn test_springer_resolver_paywall_path_returns_needs_auth() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let article_path = "/article/10.1007/s00134-020-06294-y";

    Mock::given(method("GET"))
        .and(path(article_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><body>Buy article now. Access through your institution.</body></html>",
        ))
        .mount(&mock_server)
        .await;

    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(
        SpringerResolver::with_base_urls(None, mock_server.uri(), mock_server.uri()).unwrap(),
    ));

    let ctx = ResolveContext::default();
    let input_url = format!("{}{}", mock_server.uri(), article_path);
    let err = registry
        .resolve_to_url(&input_url, InputType::Url, &ctx)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("authentication required"));
}

#[tokio::test]
async fn test_shared_default_registry_applies_specialized_priority_matrix() {
    let registry = build_default_resolver_registry(None, "test@example.com");

    let cases = [
        ("10.48550/arXiv.2301.12345", InputType::Doi, "arxiv"),
        ("10.1109/5.771073", InputType::Doi, "ieee"),
        ("10.1007/s00134-020-06294-x", InputType::Doi, "springer"),
        (
            "10.1016/j.future.2018.10.001",
            InputType::Doi,
            "sciencedirect",
        ),
    ];

    for (input, input_type, expected_first) in cases {
        let handlers = registry.find_handlers(input, input_type);
        assert!(
            !handlers.is_empty(),
            "expected handlers for input {input} ({input_type:?})"
        );
        assert_eq!(
            handlers[0].name(),
            expected_first,
            "specialized resolver should be first for {input}"
        );
    }
}

#[tokio::test]
async fn test_default_registry_falls_through_unknown_urls_cleanly() {
    let registry = build_default_resolver_registry(None, "test@example.com");
    let ctx = ResolveContext::default();

    let result = registry
        .resolve_to_url("https://example.com/unknown.pdf", InputType::Url, &ctx)
        .await
        .unwrap();

    assert_eq!(result.url, "https://example.com/unknown.pdf");
}

#[tokio::test]
async fn regression_default_registry_registers_crossref_for_generic_dois() {
    let registry = build_default_resolver_registry(None, "test@example.com");
    let handlers = registry.find_handlers("10.1234/example-doi", InputType::Doi);
    assert!(
        handlers.iter().any(|handler| handler.name() == "crossref"),
        "expected Crossref handler for non-site-specific DOI"
    );
}

#[tokio::test]
async fn regression_default_registry_skips_crossref_when_mailto_is_invalid() {
    let registry = build_default_resolver_registry(None, "invalid\nmailto@example.com");
    let handlers = registry.find_handlers("10.1234/example-doi", InputType::Doi);
    assert!(
        handlers.is_empty(),
        "invalid mailto should prevent Crossref registration; no other resolver should match generic DOI"
    );
}

#[tokio::test]
async fn test_standard_metadata_contract_keys_present_for_specialized_resolvers() {
    let mut registry = ResolverRegistry::new();
    registry.register(Box::new(ArxivResolver::new()));
    registry.register(Box::new(DirectResolver::new()));

    let ctx = ResolveContext::default();
    let result = registry
        .resolve_to_url("https://arxiv.org/abs/2301.00001", InputType::Url, &ctx)
        .await
        .unwrap();

    assert!(
        result.metadata.contains_key("source_url"),
        "specialized resolvers should always include source_url metadata"
    );
    assert!(
        result.metadata.contains_key("doi"),
        "arXiv resolver should include canonical DOI metadata"
    );

    for key in result.metadata.keys() {
        assert!(
            STANDARD_METADATA_KEYS.contains(&key.as_str())
                || key == "pmcid"
                || key == "pmid"
                || key == "ieee_arnumber",
            "unexpected metadata key in contract test: {key}"
        );
    }
}
