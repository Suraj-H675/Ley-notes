//! Explicit network boundary for Ley's first external connector slice.
//!
//! This crate fetches only already-authorized public GitHub issue/pull-request references or
//! commit-pinned text documentation.
//! `ley-core` remains responsible for connector authority, source validation, redaction,
//! persistence, deletion, and agent-egress policy. No authentication token or arbitrary URL is
//! accepted here.

use ley_core::{
    parse_public_github_reference, ExternalConnectorResourceKind, ExternalConnectorSnapshotInput,
    ExternalConnectorSource, ExternalConnectorState,
};
use serde::Deserialize;
use std::io::Read;
use std::time::Duration;
use thiserror::Error;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_GITHUB_RESPONSE_BYTES: u64 = 1_048_576;
const USER_AGENT: &str = "Ley/0.1 public-github-reference-connector";
const GITHUB_API_VERSION: &str = "2022-11-28";

#[derive(Debug, Error)]
pub enum GitHubConnectorError {
    #[error("external connector source is not a canonical supported public GitHub reference")]
    InvalidSource,
    #[error("GitHub connector request failed: {0}")]
    Request(ureq::Error),
    #[error("GitHub connector refused HTTP redirect status {0}")]
    Redirect(u16),
    #[error("GitHub connector response exceeds the {MAX_GITHUB_RESPONSE_BYTES}-byte limit")]
    ResponseTooLarge,
    #[error("could not read GitHub connector response")]
    ResponseRead,
    #[error("GitHub connector response is invalid: {0}")]
    InvalidResponse(String),
}

/// Fetches one explicitly authorized public GitHub issue, pull request, or commit-pinned document.
///
/// The canonical source is re-parsed before any request so caller-constructed structured fields
/// cannot redirect this crate to another host/path. Redirects are disabled and rejected. The
/// response is hard-bounded before JSON parsing.
pub fn fetch_public_github_reference(
    source: &ExternalConnectorSource,
) -> Result<ExternalConnectorSnapshotInput, GitHubConnectorError> {
    validate_source(source)?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout_read(READ_TIMEOUT)
        .timeout_write(WRITE_TIMEOUT)
        .redirects(0)
        .build();
    let request = agent.get(&source.api_url()).set("User-Agent", USER_AGENT);
    let request = match source.resource_kind {
        ExternalConnectorResourceKind::Issue | ExternalConnectorResourceKind::PullRequest => {
            request
                .set("Accept", "application/vnd.github+json")
                .set("X-GitHub-Api-Version", GITHUB_API_VERSION)
        }
        ExternalConnectorResourceKind::Document => request.set("Accept", "text/plain"),
    };
    let response = request.call().map_err(GitHubConnectorError::Request)?;
    if (300..400).contains(&response.status()) {
        return Err(GitHubConnectorError::Redirect(response.status()));
    }
    if let Some(content_length) = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok())
    {
        if content_length > MAX_GITHUB_RESPONSE_BYTES {
            return Err(GitHubConnectorError::ResponseTooLarge);
        }
    }
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_GITHUB_RESPONSE_BYTES.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|_| GitHubConnectorError::ResponseRead)?;
    if body.len() as u64 > MAX_GITHUB_RESPONSE_BYTES {
        return Err(GitHubConnectorError::ResponseTooLarge);
    }
    parse_github_response(source, &body)
}

fn validate_source(source: &ExternalConnectorSource) -> Result<(), GitHubConnectorError> {
    let reparsed = parse_public_github_reference(&source.canonical_url)
        .map_err(|_| GitHubConnectorError::InvalidSource)?;
    if reparsed != *source {
        return Err(GitHubConnectorError::InvalidSource);
    }
    Ok(())
}

fn parse_github_response(
    source: &ExternalConnectorSource,
    body: &[u8],
) -> Result<ExternalConnectorSnapshotInput, GitHubConnectorError> {
    match source.resource_kind {
        ExternalConnectorResourceKind::Issue => {
            let issue: GitHubIssueResponse = serde_json::from_slice(body)
                .map_err(|error| GitHubConnectorError::InvalidResponse(error.to_string()))?;
            if issue.pull_request.is_some() {
                return Err(GitHubConnectorError::InvalidResponse(
                    "the GitHub issue endpoint returned a pull request; add the canonical /pull/ URL instead"
                        .to_owned(),
                ));
            }
            Ok(ExternalConnectorSnapshotInput {
                title: issue.title,
                body: issue.body.unwrap_or_default(),
                state: Some(parse_state(&issue.state)?),
                author_login: issue.user.map(|user| user.login),
                labels: issue.labels.into_iter().map(|label| label.name).collect(),
                source_updated_at: Some(issue.updated_at),
                merged: None,
            })
        }
        ExternalConnectorResourceKind::PullRequest => {
            let pull: GitHubPullResponse = serde_json::from_slice(body)
                .map_err(|error| GitHubConnectorError::InvalidResponse(error.to_string()))?;
            Ok(ExternalConnectorSnapshotInput {
                title: pull.title,
                body: pull.body.unwrap_or_default(),
                state: Some(parse_state(&pull.state)?),
                author_login: pull.user.map(|user| user.login),
                labels: pull.labels.into_iter().map(|label| label.name).collect(),
                source_updated_at: Some(pull.updated_at),
                merged: Some(pull.merged),
            })
        }
        ExternalConnectorResourceKind::Document => {
            let content = std::str::from_utf8(body).map_err(|_| {
                GitHubConnectorError::InvalidResponse(
                    "GitHub document response must be valid UTF-8 text".to_owned(),
                )
            })?;
            let path = source.path.clone().ok_or_else(|| {
                GitHubConnectorError::InvalidResponse(
                    "GitHub document source is missing its path".to_owned(),
                )
            })?;
            Ok(ExternalConnectorSnapshotInput {
                title: path,
                body: content.to_owned(),
                state: None,
                author_login: None,
                labels: Vec::new(),
                source_updated_at: None,
                merged: None,
            })
        }
    }
}

fn parse_state(value: &str) -> Result<ExternalConnectorState, GitHubConnectorError> {
    match value {
        "open" => Ok(ExternalConnectorState::Open),
        "closed" => Ok(ExternalConnectorState::Closed),
        _ => Err(GitHubConnectorError::InvalidResponse(format!(
            "unsupported GitHub state '{value}'"
        ))),
    }
}

#[derive(Debug, Deserialize)]
struct GitHubIssueResponse {
    title: String,
    body: Option<String>,
    state: String,
    user: Option<GitHubUser>,
    #[serde(default)]
    labels: Vec<GitHubLabel>,
    updated_at: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GitHubPullResponse {
    title: String,
    body: Option<String>,
    state: String,
    user: Option<GitHubUser>,
    #[serde(default)]
    labels: Vec<GitHubLabel>,
    updated_at: String,
    merged: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_response_maps_only_bounded_core_fields() {
        let source =
            parse_public_github_reference("https://github.com/openai/ley-test/issues/42").unwrap();
        let parsed = parse_github_response(
            &source,
            br#"{
                "title":"Connector issue",
                "body":"Tracked body",
                "state":"open",
                "user":{"login":"octocat"},
                "labels":[{"name":"memory"},{"name":"connector"}],
                "updated_at":"2026-09-19T03:00:00Z",
                "extra":{"ignored":"field"}
            }"#,
        )
        .unwrap();
        assert_eq!(parsed.title, "Connector issue");
        assert_eq!(parsed.body, "Tracked body");
        assert_eq!(parsed.state, Some(ExternalConnectorState::Open));
        assert_eq!(parsed.author_login.as_deref(), Some("octocat"));
        assert_eq!(parsed.labels, vec!["memory", "connector"]);
        assert_eq!(parsed.merged, None);
    }

    #[test]
    fn issue_endpoint_refuses_pull_request_shape() {
        let source =
            parse_public_github_reference("https://github.com/openai/ley-test/issues/42").unwrap();
        let error = parse_github_response(
            &source,
            br#"{
                "title":"Actually a PR",
                "body":null,
                "state":"open",
                "user":null,
                "labels":[],
                "updated_at":"2026-09-19T03:00:00Z",
                "pull_request":{"url":"https://api.github.com/example"}
            }"#,
        )
        .unwrap_err();
        assert!(matches!(error, GitHubConnectorError::InvalidResponse(_)));
    }

    #[test]
    fn pull_response_preserves_merged_state() {
        let source =
            parse_public_github_reference("https://github.com/openai/ley-test/pull/9").unwrap();
        let parsed = parse_github_response(
            &source,
            br#"{
                "title":"Connector PR",
                "body":null,
                "state":"closed",
                "user":{"login":"octocat"},
                "labels":[],
                "updated_at":"2026-09-19T04:00:00Z",
                "merged":true
            }"#,
        )
        .unwrap();
        assert_eq!(parsed.body, "");
        assert_eq!(parsed.state, Some(ExternalConnectorState::Closed));
        assert_eq!(parsed.merged, Some(true));
    }

    #[test]
    fn document_response_preserves_only_pinned_text_content() {
        let source = parse_public_github_reference(
            "https://github.com/openai/ley-test/blob/abcdef0123456789abcdef0123456789abcdef01/docs/Guide.md",
        )
        .unwrap();
        let parsed = parse_github_response(&source, b"# Guide\n\nPinned documentation.\n").unwrap();
        assert_eq!(parsed.title, "docs/Guide.md");
        assert_eq!(parsed.body, "# Guide\n\nPinned documentation.\n");
        assert!(parsed.state.is_none());
        assert!(parsed.author_login.is_none());
        assert!(parsed.labels.is_empty());
        assert!(parsed.source_updated_at.is_none());
        assert!(parsed.merged.is_none());

        let invalid = parse_github_response(&source, &[0xff, 0xfe]).unwrap_err();
        assert!(matches!(invalid, GitHubConnectorError::InvalidResponse(_)));
    }

    #[test]
    fn structured_source_cannot_override_canonical_network_target() {
        let mut source =
            parse_public_github_reference("https://github.com/openai/ley-test/issues/42").unwrap();
        source.owner = "evil".to_owned();
        assert!(matches!(
            validate_source(&source),
            Err(GitHubConnectorError::InvalidSource)
        ));

        let mut document = parse_public_github_reference(
            "https://github.com/openai/ley-test/blob/abcdef0123456789abcdef0123456789abcdef01/README.md",
        )
        .unwrap();
        document.path = Some("docs/Other.md".to_owned());
        assert!(matches!(
            validate_source(&document),
            Err(GitHubConnectorError::InvalidSource)
        ));
    }
}
