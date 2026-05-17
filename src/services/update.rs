//! Async update-check and Overview link-action services.
//!
//! The reference application checks Nexus Mods and GitHub synchronously from the
//! Tk startup path and hides all no-update/failure states from users. This module
//! keeps that behavior fakeable and async-friendly: network and desktop work sit
//! behind injectable traits/adapters, while callers receive typed Overview update
//! state and safe action feedback.

use std::{cmp::Ordering, future::Future, pin::Pin, time::Duration};

use serde_json::Value;
use tracing::{Instrument, debug, info, info_span, warn};

use crate::{
    domain::{
        overview::{
            NEXUS_MODS_LINK, OverviewDeferredAction, OverviewDeferredActionKind,
            OverviewDeferredActionTarget, UpdateBannerState, UpdateCheckFailure, UpdateProvider,
            UpdateRelease,
        },
        settings::UpdateSource,
    },
    platform::desktop::{DesktopActionResult, DesktopActions},
    services::overview::{OverviewDesktopActionFeedback, OverviewDesktopActionOutcome},
};

/// Reference GitHub latest-release API endpoint.
pub const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/wxMichael/Collective-Modding-Toolkit/releases/latest";
/// Update request timeout used by the reference app.
pub const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

const HTTP_OK: u16 = 200;
const NEXUS_VERSION_LABEL_PREFIX: &str = "<meta property=\"twitter:label1\" content=\"Version\"";
const NEXUS_VERSION_DATA_MARKER: &str = "twitter:data1";
const CONTENT_ATTRIBUTE_MARKER: &str = "content=\"";
const MAX_NEXUS_LINES: usize = 4096;
const MAX_VERSION_TEXT_LEN: usize = 64;
const MAX_FAILURE_SUMMARY_CHARS: usize = 160;

/// Boxed async result returned by an [`UpdateCheckClient`] request.
pub type UpdateClientFuture<'a> = Pin<Box<dyn Future<Output = UpdateHttpResult> + Send + 'a>>;
/// Result returned by an [`UpdateCheckClient`] request.
pub type UpdateHttpResult = Result<UpdateHttpResponse, UpdateClientError>;

/// HTTP response data needed by the update parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateHttpResponse {
    /// HTTP status code returned by the source.
    pub status: u16,
    /// Response body for successful responses only.
    pub body: String,
}

impl UpdateHttpResponse {
    /// Creates a response with an explicit status and body.
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// Creates a successful response fixture.
    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(HTTP_OK, body)
    }
}

/// Client-side network failure categories before source-specific parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateClientErrorKind {
    /// Request exceeded the reference timeout.
    Timeout,
    /// Request could not be sent or completed.
    Request,
    /// Response body could not be decoded as text.
    BodyDecode,
}

/// Safe client failure returned by [`UpdateCheckClient`] implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateClientError {
    /// Typed client failure category.
    pub kind: UpdateClientErrorKind,
    safe_summary: String,
}

impl UpdateClientError {
    /// Creates a client error with already-safe diagnostic text.
    pub fn new(kind: UpdateClientErrorKind, safe_summary: impl Into<String>) -> Self {
        Self {
            kind,
            safe_summary: cap_failure_summary(safe_summary.into()),
        }
    }

    /// Creates a reference-compatible timeout error.
    pub fn timeout() -> Self {
        Self::new(UpdateClientErrorKind::Timeout, "update request timed out")
    }

    /// Creates a generic request failure.
    pub fn request_failed() -> Self {
        Self::new(UpdateClientErrorKind::Request, "update request failed")
    }

    /// Creates a response-body decode failure.
    pub fn body_decode_failed() -> Self {
        Self::new(
            UpdateClientErrorKind::BodyDecode,
            "update response could not be decoded",
        )
    }

    /// Returns safe diagnostic text suitable for logs or Overview diagnostics.
    pub fn safe_summary(&self) -> &str {
        &self.safe_summary
    }

    fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::timeout()
        } else {
            Self::request_failed()
        }
    }
}

/// Fakeable async boundary for update-check HTTP requests.
pub trait UpdateCheckClient: Send + Sync {
    /// Fetches the Nexus Mods project page used by the reference update check.
    fn fetch_nexus_mods(&self) -> UpdateClientFuture<'_>;

    /// Fetches the GitHub latest-release API response.
    fn fetch_github_latest_release(&self) -> UpdateClientFuture<'_>;
}

/// Reqwest-backed update-check HTTP client.
#[derive(Debug, Clone)]
pub struct RealUpdateCheckClient {
    client: reqwest::Client,
}

impl RealUpdateCheckClient {
    /// Builds a reqwest client with the reference five-second timeout.
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(UPDATE_CHECK_TIMEOUT)
            .user_agent(format!("cmt-rs/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }

    /// Creates the adapter from an already-configured reqwest client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    async fn fetch_text(
        &self,
        url: &'static str,
        headers: Option<reqwest::header::HeaderMap>,
    ) -> UpdateHttpResult {
        let mut request = self.client.get(url);
        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        let response = request
            .send()
            .await
            .map_err(UpdateClientError::from_reqwest)?;
        let status = response.status().as_u16();

        if status != HTTP_OK {
            return Ok(UpdateHttpResponse::new(status, String::new()));
        }

        let body = response
            .text()
            .await
            .map_err(|_| UpdateClientError::body_decode_failed())?;
        Ok(UpdateHttpResponse::new(status, body))
    }
}

impl UpdateCheckClient for RealUpdateCheckClient {
    fn fetch_nexus_mods(&self) -> UpdateClientFuture<'_> {
        Box::pin(async move { self.fetch_text(NEXUS_MODS_LINK, None).await })
    }

    fn fetch_github_latest_release(&self) -> UpdateClientFuture<'_> {
        Box::pin(async move {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
            );
            headers.insert(
                "X-GitHub-Api-Version",
                reqwest::header::HeaderValue::from_static("2022-11-28"),
            );
            self.fetch_text(GITHUB_LATEST_RELEASE_URL, Some(headers))
                .await
        })
    }
}

/// Source-level failure categories retained for diagnostics and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFailureKind {
    /// Source returned a non-200 status code.
    HttpStatus,
    /// Request timed out.
    Timeout,
    /// Request failed before a response could be parsed.
    Request,
    /// Response text could not be decoded.
    BodyDecode,
    /// Response JSON was malformed.
    InvalidJson,
    /// Expected version metadata was absent.
    MissingVersionMetadata,
    /// Version metadata existed but was not a supported numeric version.
    InvalidVersion,
}

impl UpdateFailureKind {
    /// Returns a stable label suitable for structured logs and tests.
    pub const fn label(self) -> &'static str {
        match self {
            Self::HttpStatus => "http-status",
            Self::Timeout => "timeout",
            Self::Request => "request",
            Self::BodyDecode => "body-decode",
            Self::InvalidJson => "invalid-json",
            Self::MissingVersionMetadata => "missing-version-metadata",
            Self::InvalidVersion => "invalid-version",
        }
    }
}

impl From<UpdateClientErrorKind> for UpdateFailureKind {
    fn from(kind: UpdateClientErrorKind) -> Self {
        match kind {
            UpdateClientErrorKind::Timeout => Self::Timeout,
            UpdateClientErrorKind::Request => Self::Request,
            UpdateClientErrorKind::BodyDecode => Self::BodyDecode,
        }
    }
}

/// Per-provider update-check outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateSourceOutcome {
    /// Provider was intentionally skipped by the selected update source.
    Skipped {
        /// Safe reason for skipping.
        reason: String,
    },
    /// Provider completed successfully and found no newer version.
    NoUpdate {
        /// Parsed remote version when present.
        remote_version: Option<String>,
    },
    /// Provider reported a newer version.
    Available {
        /// Banner-ready release metadata.
        release: UpdateRelease,
    },
    /// Provider failed in the reference-compatible silent path.
    Failed {
        /// Typed failure category.
        kind: UpdateFailureKind,
        /// Safe failure summary for diagnostics/logs.
        failure: UpdateCheckFailure,
    },
}

/// Source-specific update-check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateSourceResult {
    /// Provider associated with the result.
    pub provider: UpdateProvider,
    /// Provider outcome.
    pub outcome: UpdateSourceOutcome,
}

impl UpdateSourceResult {
    /// Creates a skipped source result.
    pub fn skipped(provider: UpdateProvider, reason: impl Into<String>) -> Self {
        Self {
            provider,
            outcome: UpdateSourceOutcome::Skipped {
                reason: reason.into(),
            },
        }
    }

    /// Creates a no-update source result.
    pub fn no_update(provider: UpdateProvider, remote_version: Option<String>) -> Self {
        Self {
            provider,
            outcome: UpdateSourceOutcome::NoUpdate { remote_version },
        }
    }

    /// Creates an available-update source result.
    pub fn available(provider: UpdateProvider, version: impl Into<String>) -> Self {
        Self {
            provider,
            outcome: UpdateSourceOutcome::Available {
                release: UpdateRelease::new(provider, version),
            },
        }
    }

    /// Creates a silent-failure source result.
    pub fn failed(
        provider: UpdateProvider,
        kind: UpdateFailureKind,
        summary: impl Into<String>,
    ) -> Self {
        let failure = UpdateCheckFailure::new(provider, cap_failure_summary(summary.into()));
        Self {
            provider,
            outcome: UpdateSourceOutcome::Failed { kind, failure },
        }
    }
}

/// Full update-check report for the selected source setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckReport {
    /// Selected source from settings.
    pub selected_source: UpdateSource,
    /// Source-specific outcomes in reference display/request order.
    pub source_results: Vec<UpdateSourceResult>,
}

impl UpdateCheckReport {
    /// Creates an empty report for a selected source.
    pub fn new(selected_source: UpdateSource) -> Self {
        Self {
            selected_source,
            source_results: Vec::new(),
        }
    }

    /// Returns banner-ready newer releases in source order.
    pub fn releases(&self) -> Vec<UpdateRelease> {
        self.source_results
            .iter()
            .filter_map(|result| match &result.outcome {
                UpdateSourceOutcome::Available { release } => Some(release.clone()),
                UpdateSourceOutcome::Skipped { .. }
                | UpdateSourceOutcome::NoUpdate { .. }
                | UpdateSourceOutcome::Failed { .. } => None,
            })
            .collect()
    }

    /// Returns silent update-check failures in source order.
    pub fn failures(&self) -> Vec<UpdateCheckFailure> {
        self.source_results
            .iter()
            .filter_map(|result| match &result.outcome {
                UpdateSourceOutcome::Failed { failure, .. } => Some(failure.clone()),
                UpdateSourceOutcome::Skipped { .. }
                | UpdateSourceOutcome::NoUpdate { .. }
                | UpdateSourceOutcome::Available { .. } => None,
            })
            .collect()
    }

    /// Converts the report into the worker state consumed by Overview diagnostics.
    pub fn overview_state(&self) -> crate::services::overview::OverviewUpdateCheckState {
        if matches!(self.selected_source, UpdateSource::None) {
            return crate::services::overview::OverviewUpdateCheckState::NotChecked;
        }

        let releases = self.releases();
        if !releases.is_empty() {
            return crate::services::overview::OverviewUpdateCheckState::Completed { releases };
        }

        let failures = self.failures();
        if failures.is_empty() {
            crate::services::overview::OverviewUpdateCheckState::Completed {
                releases: Vec::new(),
            }
        } else {
            crate::services::overview::OverviewUpdateCheckState::FailedSilently { failures }
        }
    }

    /// Converts the report directly into a reference-compatible banner state.
    pub fn banner_state(&self) -> UpdateBannerState {
        if matches!(self.selected_source, UpdateSource::None) {
            return UpdateBannerState::Disabled;
        }

        match self.overview_state() {
            crate::services::overview::OverviewUpdateCheckState::NotChecked => {
                UpdateBannerState::NotChecked {
                    selected_source: self.selected_source,
                }
            }
            crate::services::overview::OverviewUpdateCheckState::Checking => {
                UpdateBannerState::Checking {
                    selected_source: self.selected_source,
                }
            }
            crate::services::overview::OverviewUpdateCheckState::Completed { releases } => {
                UpdateBannerState::available_or_no_update(self.selected_source, releases)
            }
            crate::services::overview::OverviewUpdateCheckState::FailedSilently { failures } => {
                UpdateBannerState::failed_silently(self.selected_source, failures)
            }
        }
    }
}

/// Async update-check service that preserves the reference source-selection semantics.
#[derive(Debug, Clone)]
pub struct UpdateCheckService<C> {
    client: C,
    current_version: String,
}

impl<C> UpdateCheckService<C> {
    /// Creates a service using the crate package version as the current version.
    pub fn new(client: C) -> Self {
        Self::with_current_version(client, env!("CARGO_PKG_VERSION"))
    }

    /// Creates a service with a test-injectable current version.
    pub fn with_current_version(client: C, current_version: impl Into<String>) -> Self {
        Self {
            client,
            current_version: current_version.into(),
        }
    }
}

impl<C: UpdateCheckClient> UpdateCheckService<C> {
    /// Checks the configured update source and returns a silent/banner-ready report.
    pub async fn check(&self, selected_source: UpdateSource) -> UpdateCheckReport {
        let span = info_span!(
            "overview_update_check",
            update_source = selected_source.as_wire_value()
        );
        self.check_inner(selected_source).instrument(span).await
    }

    async fn check_inner(&self, selected_source: UpdateSource) -> UpdateCheckReport {
        let mut report = UpdateCheckReport::new(selected_source);
        let providers = selected_providers(selected_source);

        info!(
            update_source = selected_source.as_wire_value(),
            provider_count = providers.len(),
            "overview update check started"
        );

        if providers.is_empty() {
            report.source_results.push(UpdateSourceResult::skipped(
                UpdateProvider::NexusMods,
                "update checks disabled",
            ));
            report.source_results.push(UpdateSourceResult::skipped(
                UpdateProvider::Github,
                "update checks disabled",
            ));
            debug!("overview update check skipped because source is disabled");
            return report;
        }

        for provider in providers {
            let result = self.check_provider(provider).await;
            match &result.outcome {
                UpdateSourceOutcome::Available { release } => info!(
                    provider = provider.label(),
                    version = release.version.as_str(),
                    "overview update available"
                ),
                UpdateSourceOutcome::NoUpdate { remote_version } => debug!(
                    provider = provider.label(),
                    remote_version = remote_version.as_deref().unwrap_or("unknown"),
                    "overview update source has no newer version"
                ),
                UpdateSourceOutcome::Failed { kind, failure } => warn!(
                    provider = provider.label(),
                    failure_kind = kind.label(),
                    summary = failure.summary.as_str(),
                    "overview update check failed silently"
                ),
                UpdateSourceOutcome::Skipped { reason } => debug!(
                    provider = provider.label(),
                    reason = reason.as_str(),
                    "overview update source skipped"
                ),
            }
            report.source_results.push(result);
        }

        info!(
            update_source = selected_source.as_wire_value(),
            releases = report.releases().len(),
            failures = report.failures().len(),
            "overview update check completed"
        );
        report
    }

    async fn check_provider(&self, provider: UpdateProvider) -> UpdateSourceResult {
        debug!(
            provider = provider.label(),
            "overview update source request started"
        );
        let response = match provider {
            UpdateProvider::NexusMods => self.client.fetch_nexus_mods().await,
            UpdateProvider::Github => self.client.fetch_github_latest_release().await,
        };

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return UpdateSourceResult::failed(
                    provider,
                    error.kind.into(),
                    error.safe_summary().to_owned(),
                );
            }
        };

        if response.status != HTTP_OK {
            return UpdateSourceResult::failed(
                provider,
                UpdateFailureKind::HttpStatus,
                format!("update source returned HTTP status {}", response.status),
            );
        }

        match provider {
            UpdateProvider::NexusMods => self.check_nexus_body(&response.body),
            UpdateProvider::Github => self.check_github_body(&response.body),
        }
    }

    fn check_nexus_body(&self, body: &str) -> UpdateSourceResult {
        let provider = UpdateProvider::NexusMods;
        match parse_nexus_version(body) {
            Ok(version) => compare_remote_version(provider, &version, &self.current_version),
            Err(kind) => UpdateSourceResult::failed(provider, kind, failure_summary(kind)),
        }
    }

    fn check_github_body(&self, body: &str) -> UpdateSourceResult {
        let provider = UpdateProvider::Github;
        let json: Value = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(_) => {
                return UpdateSourceResult::failed(
                    provider,
                    UpdateFailureKind::InvalidJson,
                    failure_summary(UpdateFailureKind::InvalidJson),
                );
            }
        };

        let Some(version) = json
            .get("tag_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|version| !version.is_empty())
        else {
            return UpdateSourceResult::failed(
                provider,
                UpdateFailureKind::MissingVersionMetadata,
                failure_summary(UpdateFailureKind::MissingVersionMetadata),
            );
        };

        compare_remote_version(provider, version, &self.current_version)
    }
}

/// Executes Overview path/URL actions through an injectable desktop adapter.
#[derive(Debug, Clone)]
pub struct OverviewLinkService<D> {
    desktop: D,
}

impl<D> OverviewLinkService<D> {
    /// Creates an Overview link/action executor.
    pub fn new(desktop: D) -> Self {
        Self { desktop }
    }
}

impl<D: DesktopActions> OverviewLinkService<D> {
    /// Executes an enabled URL/path action and returns Overview-safe feedback.
    pub fn execute(&self, action: &OverviewDeferredAction) -> OverviewDesktopActionFeedback {
        if !action.enabled {
            warn!(action = ?action.kind, "overview desktop action rejected because it is disabled");
            return OverviewDesktopActionFeedback::failed(action.kind, "Action is disabled.");
        }

        let (target_type, result) = match &action.target {
            OverviewDeferredActionTarget::Url(url) => {
                info!(action = ?action.kind, target_type = "url", "overview desktop action started");
                ("url", self.desktop.open_url(url))
            }
            OverviewDeferredActionTarget::Path(path) => {
                info!(action = ?action.kind, target_type = "path", "overview desktop action started");
                ("path", self.desktop.open_path(path))
            }
            OverviewDeferredActionTarget::Internal => {
                warn!(action = ?action.kind, "overview desktop action rejected because it is internal");
                return OverviewDesktopActionFeedback::failed(
                    action.kind,
                    "Action does not have an external target.",
                );
            }
        };

        desktop_result_to_feedback(action.kind, target_type, result)
    }
}

fn desktop_result_to_feedback(
    action: OverviewDeferredActionKind,
    target_type: &'static str,
    result: DesktopActionResult,
) -> OverviewDesktopActionFeedback {
    if result.is_success() {
        info!(action = ?action, target_type, "overview desktop action completed");
        return OverviewDesktopActionFeedback::succeeded(action);
    }

    warn!(
        action = ?action,
        target_type,
        failure_kind = ?result.failure_kind(),
        "overview desktop action failed"
    );
    OverviewDesktopActionFeedback {
        action,
        outcome: OverviewDesktopActionOutcome::Failed {
            safe_message: result.safe_message().to_owned(),
        },
    }
}

fn selected_providers(selected_source: UpdateSource) -> Vec<UpdateProvider> {
    match selected_source {
        UpdateSource::Both => vec![UpdateProvider::NexusMods, UpdateProvider::Github],
        UpdateSource::Github => vec![UpdateProvider::Github],
        UpdateSource::Nexus => vec![UpdateProvider::NexusMods],
        UpdateSource::None => Vec::new(),
    }
}

fn parse_nexus_version(body: &str) -> Result<String, UpdateFailureKind> {
    let mut use_next_version_meta = false;

    for line in body.lines().take(MAX_NEXUS_LINES) {
        let trimmed = line.trim_start();
        if use_next_version_meta {
            return extract_content_attribute(trimmed)
                .ok_or(UpdateFailureKind::MissingVersionMetadata);
        }

        if trimmed.starts_with(NEXUS_VERSION_LABEL_PREFIX) {
            use_next_version_meta = true;
            continue;
        }

        if trimmed.contains(NEXUS_VERSION_DATA_MARKER) {
            return extract_content_attribute(trimmed)
                .ok_or(UpdateFailureKind::MissingVersionMetadata);
        }
    }

    Err(UpdateFailureKind::MissingVersionMetadata)
}

fn extract_content_attribute(line: &str) -> Option<String> {
    let value_start = line.find(CONTENT_ATTRIBUTE_MARKER)? + CONTENT_ATTRIBUTE_MARKER.len();
    let value_end = line[value_start..].find('"')?;
    let value = line[value_start..value_start + value_end].trim();
    (!value.is_empty() && value.chars().count() <= MAX_VERSION_TEXT_LEN).then(|| value.to_owned())
}

fn compare_remote_version(
    provider: UpdateProvider,
    remote_version: &str,
    current_version: &str,
) -> UpdateSourceResult {
    let remote = match ParsedVersion::parse(remote_version) {
        Ok(version) => version,
        Err(_) => {
            return UpdateSourceResult::failed(
                provider,
                UpdateFailureKind::InvalidVersion,
                failure_summary(UpdateFailureKind::InvalidVersion),
            );
        }
    };

    let current = match ParsedVersion::parse(current_version) {
        Ok(version) => version,
        Err(_) => {
            return UpdateSourceResult::failed(
                provider,
                UpdateFailureKind::InvalidVersion,
                "current application version metadata is invalid",
            );
        }
    };

    if remote.cmp_numeric(&current) == Ordering::Greater {
        UpdateSourceResult::available(provider, remote.normalized)
    } else {
        UpdateSourceResult::no_update(provider, Some(remote.normalized))
    }
}

fn failure_summary(kind: UpdateFailureKind) -> &'static str {
    match kind {
        UpdateFailureKind::HttpStatus => "update source returned an unsuccessful HTTP status",
        UpdateFailureKind::Timeout => "update request timed out",
        UpdateFailureKind::Request => "update request failed",
        UpdateFailureKind::BodyDecode => "update response could not be decoded",
        UpdateFailureKind::InvalidJson => "update metadata could not be parsed",
        UpdateFailureKind::MissingVersionMetadata => "update version metadata was not found",
        UpdateFailureKind::InvalidVersion => "update version metadata was invalid",
    }
}

fn cap_failure_summary(summary: String) -> String {
    let mut capped = String::with_capacity(summary.len().min(MAX_FAILURE_SUMMARY_CHARS));
    for (index, character) in summary.chars().enumerate() {
        if index >= MAX_FAILURE_SUMMARY_CHARS {
            capped.push('…');
            return capped;
        }
        capped.push(character);
    }
    capped
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    components: Vec<u64>,
    normalized: String,
}

impl ParsedVersion {
    fn parse(source: &str) -> Result<Self, ()> {
        let trimmed = source.trim();
        let without_prefix = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);

        if without_prefix.is_empty() || without_prefix.chars().count() > MAX_VERSION_TEXT_LEN {
            return Err(());
        }

        let mut components = Vec::new();
        for part in without_prefix.split('.') {
            if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
                return Err(());
            }
            components.push(part.parse::<u64>().map_err(|_| ())?);
        }

        if components.is_empty() {
            return Err(());
        }

        let normalized = components
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(".");
        Ok(Self {
            components,
            normalized,
        })
    }

    fn cmp_numeric(&self, other: &Self) -> Ordering {
        let component_count = self.components.len().max(other.components.len());
        for index in 0..component_count {
            let left = self.components.get(index).copied().unwrap_or_default();
            let right = other.components.get(index).copied().unwrap_or_default();
            match left.cmp(&right) {
                Ordering::Equal => continue,
                ordering => return ordering,
            }
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod overview_update_tests {
    use std::{
        collections::{BTreeMap, VecDeque},
        path::Path,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::{
        domain::overview::GITHUB_LINK,
        platform::{PlatformError, PlatformErrorKind, PlatformOperation},
    };

    #[derive(Debug, Clone, Default)]
    struct FakeUpdateCheckClient {
        nexus: Arc<Mutex<VecDeque<UpdateHttpResult>>>,
        github: Arc<Mutex<VecDeque<UpdateHttpResult>>>,
        calls: Arc<Mutex<Vec<UpdateProvider>>>,
    }

    impl FakeUpdateCheckClient {
        fn new() -> Self {
            Self::default()
        }

        fn with_nexus_response(self, response: UpdateHttpResult) -> Self {
            self.nexus
                .lock()
                .expect("fake nexus response queue should be available")
                .push_back(response);
            self
        }

        fn with_github_response(self, response: UpdateHttpResult) -> Self {
            self.github
                .lock()
                .expect("fake github response queue should be available")
                .push_back(response);
            self
        }

        fn calls(&self) -> Vec<UpdateProvider> {
            self.calls
                .lock()
                .expect("fake call log should be available")
                .clone()
        }

        fn next_response(&self, provider: UpdateProvider) -> UpdateHttpResult {
            self.calls
                .lock()
                .expect("fake call log should be available")
                .push(provider);

            let queue = match provider {
                UpdateProvider::NexusMods => &self.nexus,
                UpdateProvider::Github => &self.github,
            };
            queue
                .lock()
                .expect("fake response queue should be available")
                .pop_front()
                .unwrap_or_else(|| Err(UpdateClientError::request_failed()))
        }
    }

    impl UpdateCheckClient for FakeUpdateCheckClient {
        fn fetch_nexus_mods(&self) -> UpdateClientFuture<'_> {
            Box::pin(async move { self.next_response(UpdateProvider::NexusMods) })
        }

        fn fetch_github_latest_release(&self) -> UpdateClientFuture<'_> {
            Box::pin(async move { self.next_response(UpdateProvider::Github) })
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeDesktopActions {
        failures: BTreeMap<(PlatformOperation, String), PlatformErrorKind>,
    }

    impl FakeDesktopActions {
        fn fail(
            mut self,
            operation: PlatformOperation,
            target: impl Into<String>,
            kind: PlatformErrorKind,
        ) -> Self {
            self.failures.insert((operation, target.into()), kind);
            self
        }

        fn run(&self, operation: PlatformOperation, target: String) -> DesktopActionResult {
            if let Some(kind) = self.failures.get(&(operation, target.clone())) {
                DesktopActionResult::failure(PlatformError::new(
                    operation,
                    target,
                    *kind,
                    format!("{} failed.", operation.label()),
                ))
            } else {
                DesktopActionResult::success(operation, target)
            }
        }
    }

    impl DesktopActions for FakeDesktopActions {
        fn open_url(&self, url: &str) -> DesktopActionResult {
            self.run(PlatformOperation::OpenUrl, url.to_owned())
        }

        fn open_path(&self, path: &Path) -> DesktopActionResult {
            self.run(PlatformOperation::OpenPath, path.display().to_string())
        }

        fn launch_tool(&self, executable: &Path, _args: &[String]) -> DesktopActionResult {
            self.run(
                PlatformOperation::LaunchTool,
                executable.display().to_string(),
            )
        }
    }

    fn service(client: FakeUpdateCheckClient) -> UpdateCheckService<FakeUpdateCheckClient> {
        UpdateCheckService::with_current_version(client, "0.6.1")
    }

    fn nexus_page(version: &str) -> String {
        format!(
            r#"<html>
<meta property="twitter:label1" content="Version">
<meta property="twitter:data1" content="{version}">
</html>"#
        )
    }

    fn github_json(version: &str) -> String {
        format!(r#"{{"tag_name":"{version}"}}"#)
    }

    #[tokio::test]
    async fn overview_update_source_none_skips_all_clients() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok(nexus_page("9.9.9"))))
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("9.9.9"))));
        let checker = service(client.clone());

        let report = checker.check(UpdateSource::None).await;

        assert!(client.calls().is_empty());
        assert!(report.releases().is_empty());
        assert!(report.failures().is_empty());
        assert!(matches!(report.banner_state(), UpdateBannerState::Disabled));
        assert!(
            report
                .source_results
                .iter()
                .all(|result| { matches!(result.outcome, UpdateSourceOutcome::Skipped { .. }) })
        );
    }

    #[tokio::test]
    async fn overview_update_nexus_only_calls_nexus_and_reports_newer_version() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok(nexus_page("0.7.0"))))
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("0.8.0"))));
        let checker = service(client.clone());

        let report = checker.check(UpdateSource::Nexus).await;

        assert_eq!(client.calls(), vec![UpdateProvider::NexusMods]);
        assert_eq!(report.releases()[0].provider, UpdateProvider::NexusMods);
        assert_eq!(report.releases()[0].version, "0.7.0");
        assert!(report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_github_only_calls_github_and_reports_newer_version() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok(nexus_page("0.8.0"))))
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("v0.7.1"))));
        let checker = service(client.clone());

        let report = checker.check(UpdateSource::Github).await;

        assert_eq!(client.calls(), vec![UpdateProvider::Github]);
        assert_eq!(report.releases()[0].provider, UpdateProvider::Github);
        assert_eq!(report.releases()[0].version, "0.7.1");
        assert_eq!(report.releases()[0].display_label(), "v0.7.1 (GitHub)");
        assert!(report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_both_sources_checks_nexus_then_github_without_retries() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok(nexus_page("0.7.0"))))
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("0.7.1"))));
        let checker = service(client.clone());

        let report = checker.check(UpdateSource::Both).await;

        assert_eq!(
            client.calls(),
            vec![UpdateProvider::NexusMods, UpdateProvider::Github]
        );
        let releases = report.releases();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].display_label(), "v0.7.0 (NexusMods)");
        assert_eq!(releases[1].display_label(), "v0.7.1 (GitHub)");
        assert!(report.failures().is_empty());
    }

    #[tokio::test]
    async fn overview_update_newer_version_creates_visible_banner_metadata() {
        let client = FakeUpdateCheckClient::new()
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("0.7.0"))));
        let checker = service(client);

        let report = checker.check(UpdateSource::Github).await;

        let UpdateBannerState::Available {
            selected_source,
            releases,
        } = report.banner_state()
        else {
            panic!("newer version should create available banner");
        };
        assert_eq!(selected_source, UpdateSource::Github);
        assert_eq!(
            releases[0].action.target,
            OverviewDeferredActionTarget::Url(GITHUB_LINK.to_owned())
        );
        assert_eq!(releases[0].display_label(), "v0.7.0 (GitHub)");
    }

    #[tokio::test]
    async fn overview_update_equal_or_older_versions_are_silent_no_update() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok(nexus_page("0.6.1"))))
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("0.6.0"))));
        let checker = service(client);

        let report = checker.check(UpdateSource::Both).await;

        assert!(report.releases().is_empty());
        assert!(report.failures().is_empty());
        assert!(matches!(
            report.banner_state(),
            UpdateBannerState::NoUpdate {
                selected_source: UpdateSource::Both
            }
        ));
        assert!(!report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_malformed_github_json_is_silent_with_diagnostics() {
        let client = FakeUpdateCheckClient::new()
            .with_github_response(Ok(UpdateHttpResponse::ok("{not-json")));
        let checker = service(client);

        let report = checker.check(UpdateSource::Github).await;

        assert!(report.releases().is_empty());
        assert_eq!(
            report.failures()[0].summary,
            "update metadata could not be parsed"
        );
        let UpdateSourceOutcome::Failed { kind, .. } = &report.source_results[0].outcome else {
            panic!("malformed JSON should be a silent source failure");
        };
        assert_eq!(*kind, UpdateFailureKind::InvalidJson);
        assert!(!report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_nexus_page_without_version_meta_is_silent_with_diagnostics() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Ok(UpdateHttpResponse::ok("<html>No version here</html>")));
        let checker = service(client);

        let report = checker.check(UpdateSource::Nexus).await;

        assert!(report.releases().is_empty());
        assert_eq!(
            report.failures()[0].summary,
            "update version metadata was not found"
        );
        assert!(!report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_invalid_version_string_is_silent_with_diagnostics() {
        let client = FakeUpdateCheckClient::new()
            .with_github_response(Ok(UpdateHttpResponse::ok(github_json("not-a-version"))));
        let checker = service(client);

        let report = checker.check(UpdateSource::Github).await;

        assert!(report.releases().is_empty());
        assert_eq!(
            report.failures()[0].summary,
            "update version metadata was invalid"
        );
        assert!(!report.banner_state().is_visible());
    }

    #[tokio::test]
    async fn overview_update_timeout_and_http_failure_are_silent_with_diagnostics() {
        let client = FakeUpdateCheckClient::new()
            .with_nexus_response(Err(UpdateClientError::timeout()))
            .with_github_response(Ok(UpdateHttpResponse::new(500, String::new())));
        let checker = service(client);

        let report = checker.check(UpdateSource::Both).await;

        assert!(report.releases().is_empty());
        let failures = report.failures();
        assert_eq!(failures[0].summary, "update request timed out");
        assert_eq!(
            failures[1].summary,
            "update source returned HTTP status 500"
        );
        assert!(!report.banner_state().is_visible());
    }

    #[test]
    fn overview_update_desktop_action_failure_feedback_is_safe_for_links_and_paths() {
        let desktop = FakeDesktopActions::default()
            .fail(
                PlatformOperation::OpenUrl,
                GITHUB_LINK,
                PlatformErrorKind::CommandFailed,
            )
            .fail(
                PlatformOperation::OpenPath,
                r"C:\Games\Fallout 4",
                PlatformErrorKind::NotFound,
            );
        let service = OverviewLinkService::new(desktop);

        let release = UpdateRelease::new(UpdateProvider::Github, "0.7.0");
        let url_feedback = service.execute(&release.action);
        assert_eq!(
            url_feedback.action,
            OverviewDeferredActionKind::OpenUpdateProvider(UpdateProvider::Github)
        );
        assert_eq!(
            url_feedback.outcome,
            OverviewDesktopActionOutcome::Failed {
                safe_message: "URL open failed.".to_owned()
            }
        );

        let path_action = OverviewDeferredAction::open_path(
            OverviewDeferredActionKind::OpenGamePath,
            "Game Path",
            r"C:\Games\Fallout 4",
        );
        let path_feedback = service.execute(&path_action);
        assert_eq!(
            path_feedback.action,
            OverviewDeferredActionKind::OpenGamePath
        );
        assert_eq!(
            path_feedback.outcome,
            OverviewDesktopActionOutcome::Failed {
                safe_message: "Path open failed.".to_owned()
            }
        );
    }

    #[test]
    fn overview_update_version_parser_normalizes_and_compares_numeric_parts() {
        let parsed = ParsedVersion::parse("v01.002.3").expect("version should parse");
        assert_eq!(parsed.normalized, "1.2.3");
        assert_eq!(
            ParsedVersion::parse("0.10.0")
                .expect("version should parse")
                .cmp_numeric(&ParsedVersion::parse("0.9.9").expect("version should parse")),
            Ordering::Greater
        );
        assert!(ParsedVersion::parse("0.7.beta").is_err());
    }
}
