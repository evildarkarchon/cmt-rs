//! Fail-closed Scanner Auto-Fix service seam.
//!
//! The reference registry in `CMT/src/autofixes.py` is empty. The production
//! constructor here preserves that behavior while exposing a typed registry,
//! closure-free support catalog, planning, and execution seam for future fixes.

use std::{collections::BTreeMap, fmt, path::Path};

use tracing::{info, info_span, warn};

use crate::{
    domain::{
        autofix::{
            AutoFixCompletion, AutoFixConfirmation, AutoFixOperationKey, AutoFixPlanPreview,
            AutoFixRejection, AutoFixRejectionReason, AutoFixRequest, AutoFixResultDetail,
            AutoFixRevalidationPlan, AutoFixSelectionIdentity, AutoFixStatus, AutoFixStatusKind,
        },
        scanner::{ScannerResult, ScannerScanSnapshot, ScannerSolutionKind},
    },
    platform::filesystem::Filesystem,
};

const SAFE_UNAVAILABLE: &str = "Auto-Fix is not available for this result.";
const SAFE_SCAN_CHANGED: &str =
    "Auto-Fix could not run because the scan results changed. Scan again and retry.";
const SAFE_RESULT_MISSING: &str =
    "Auto-Fix could not find the selected result. Select a result and retry.";
const SAFE_TARGET_MISSING: &str = "Auto-Fix could not find a target path for this result.";
const SAFE_TARGET_MISMATCH: &str =
    "Auto-Fix could not safely match the target path for this result.";
const SAFE_OPERATION_MISMATCH: &str = "Auto-Fix could not safely match the requested operation.";
const SAFE_CONFIRMATION_REQUIRED: &str = "Auto-Fix needs confirmation before making changes.";
const SAFE_CONFIRMATION_DECLINED: &str = "Auto-Fix was cancelled before making changes.";
const SAFE_VALIDATION_FAILED: &str =
    "Auto-Fix could not safely validate this result before making changes.";

/// Request to preview an Auto-Fix action for one selected scanner result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixPlanRequest {
    /// Scan id the selected result belongs to.
    pub scan_id: u64,
    /// Flat scanner result index selected by the controller.
    pub result_index: usize,
    /// Identity captured when the row was selected.
    pub selection_identity: AutoFixSelectionIdentity,
}

impl AutoFixPlanRequest {
    /// Creates a plan request from the selected scan/result identity.
    pub fn new(
        scan_id: u64,
        result_index: usize,
        selection_identity: AutoFixSelectionIdentity,
    ) -> Self {
        Self {
            scan_id,
            result_index,
            selection_identity,
        }
    }
}

/// Result of planning an Auto-Fix action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoFixPlanResult {
    /// The selected result has a registered operation and can be confirmed/executed.
    Planned(AutoFixPlanPreview),
    /// The selected result failed closed before any operation could run.
    Rejected(AutoFixRejection),
}

/// Result of executing an Auto-Fix request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoFixServiceResult {
    /// A registered operation ran and returned success or controlled failure.
    Completed(AutoFixCompletion),
    /// The request failed closed before mutation.
    Rejected(AutoFixRejection),
}

/// UI/controller-safe metadata for one registered Auto-Fix operation.
///
/// This type intentionally contains no operation closures. Controllers can use
/// it to decide whether to show an Auto-Fix affordance while execution remains
/// private to [`AutoFixService`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixOperationSupport {
    /// Typed registry key derived from retained scanner solution identity.
    pub operation_key: AutoFixOperationKey,
    /// User-safe action label.
    pub safe_label: String,
    /// User-safe preview text.
    pub safe_preview: String,
    /// Whether a selected filesystem target is required.
    pub requires_target_path: bool,
    /// Whether explicit confirmation is required.
    pub confirmation_required: bool,
    /// Optional user-safe confirmation prompt.
    pub confirmation_prompt: Option<String>,
}

impl AutoFixOperationSupport {
    /// Creates operation support metadata with no extra requirements.
    pub fn new(
        operation_key: AutoFixOperationKey,
        safe_label: impl Into<String>,
        safe_preview: impl Into<String>,
    ) -> Self {
        Self {
            operation_key,
            safe_label: safe_label.into(),
            safe_preview: safe_preview.into(),
            requires_target_path: false,
            confirmation_required: false,
            confirmation_prompt: None,
        }
    }

    /// Marks this operation as requiring a selected target path.
    pub fn with_required_target_path(mut self) -> Self {
        self.requires_target_path = true;
        self
    }

    /// Marks this operation as requiring explicit confirmation.
    pub fn with_confirmation(mut self, prompt: impl Into<String>) -> Self {
        self.confirmation_required = true;
        self.confirmation_prompt = Some(prompt.into());
        self
    }

    /// Returns the stable operation id for logs and model payloads.
    pub const fn operation_id(&self) -> &'static str {
        self.operation_key.as_id()
    }
}

/// Closure-free support catalog projected from an Auto-Fix registry.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AutoFixSupportCatalog {
    operations: BTreeMap<AutoFixOperationKey, AutoFixOperationSupport>,
}

impl AutoFixSupportCatalog {
    /// Creates an empty support catalog.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns true when no operations are registered.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Returns the number of registered support entries.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Iterates registered operations in deterministic key order.
    pub fn entries(&self) -> impl Iterator<Item = &AutoFixOperationSupport> {
        self.operations.values()
    }

    /// Returns support metadata for a typed operation key.
    pub fn support_for_key(
        &self,
        operation_key: AutoFixOperationKey,
    ) -> Option<&AutoFixOperationSupport> {
        self.operations.get(&operation_key)
    }

    /// Returns support metadata for a retained typed scanner solution kind.
    pub fn support_for_solution_kind(
        &self,
        solution_kind: &ScannerSolutionKind,
    ) -> Option<&AutoFixOperationSupport> {
        solution_kind
            .auto_fix_operation_key()
            .and_then(|operation_key| self.support_for_key(operation_key))
    }

    /// Returns support metadata for a scanner result without matching display text.
    pub fn support_for_result(&self, result: &ScannerResult) -> Option<&AutoFixOperationSupport> {
        result
            .auto_fix_operation_key()
            .and_then(|operation_key| self.support_for_key(operation_key))
    }

    fn from_registry(registry: &AutoFixRegistry) -> Self {
        Self {
            operations: registry
                .operations
                .iter()
                .map(|(operation_key, operation)| (*operation_key, operation.support.clone()))
                .collect(),
        }
    }
}

/// Successful result returned by a registered Auto-Fix operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixOperationSuccess {
    /// User-safe one-line summary.
    pub safe_summary: String,
    /// User-safe details for inline feedback or Auto-Fix Results.
    pub details: String,
    /// Non-user-facing diagnostic detail for logs/tests.
    pub diagnostic: Option<String>,
}

impl AutoFixOperationSuccess {
    /// Creates a successful operation result with safe text.
    pub fn new(safe_summary: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            safe_summary: safe_summary.into(),
            details: details.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail while preserving safe user text.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

/// Failure returned by operation preconditions or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFixOperationFailure {
    /// User-safe one-line failure summary.
    pub safe_message: String,
    /// User-safe details for inline feedback or Auto-Fix Results.
    pub details: String,
    /// Non-user-facing diagnostic detail for logs/tests.
    pub diagnostic: Option<String>,
}

impl AutoFixOperationFailure {
    /// Creates an operation failure with safe text.
    pub fn new(safe_message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            safe_message: safe_message.into(),
            details: details.into(),
            diagnostic: None,
        }
    }

    /// Adds diagnostic detail while preserving safe user text.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

/// Context passed to operation preconditions and execution.
///
/// The context is built only after scan id, result index, row identity, typed
/// operation, target path, confirmation, and revalidation checks pass.
pub struct AutoFixOperationContext<'a> {
    /// Scan id being fixed.
    pub scan_id: u64,
    /// Flat scanner result index being fixed.
    pub result_index: usize,
    /// Registered operation key being executed.
    pub operation_key: AutoFixOperationKey,
    /// Identity validated immediately before mutation.
    pub selection_identity: &'a AutoFixSelectionIdentity,
    /// Selected scanner result.
    pub result: &'a ScannerResult,
    /// Optional target path selected for mutation.
    pub target_path: Option<&'a Path>,
    /// Filesystem adapter for bounded precondition checks.
    pub filesystem: &'a dyn Filesystem,
    /// Confirmation payload, when supplied.
    pub confirmation: Option<&'a AutoFixConfirmation>,
}

/// Executable behavior for a registered Auto-Fix operation.
///
/// Implementations should keep mutation in [`Self::execute`].
/// [`Self::validate_preconditions`] is called first and must be side-effect-free.
pub trait AutoFixOperationRunner: Send + Sync {
    /// Revalidates operation-specific facts before mutation.
    fn validate_preconditions(
        &self,
        _context: &AutoFixOperationContext<'_>,
    ) -> Result<(), AutoFixOperationFailure> {
        Ok(())
    }

    /// Runs the mutating Auto-Fix operation after all validations pass.
    fn execute(
        &self,
        context: &AutoFixOperationContext<'_>,
    ) -> Result<AutoFixOperationSuccess, AutoFixOperationFailure>;
}

struct AutoFixOperation {
    support: AutoFixOperationSupport,
    runner: Box<dyn AutoFixOperationRunner>,
}

/// Injectable registry of executable Auto-Fix operations.
///
/// The default/production registry is empty to match `AUTO_FIXES = {}`.
#[derive(Default)]
pub struct AutoFixRegistry {
    operations: BTreeMap<AutoFixOperationKey, AutoFixOperation>,
}

impl AutoFixRegistry {
    /// Creates an empty production registry.
    pub fn production() -> Self {
        Self::default()
    }

    /// Creates an empty registry for tests or future composition.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Registers an executable operation and returns any replaced support metadata.
    pub fn register<R>(
        &mut self,
        support: AutoFixOperationSupport,
        runner: R,
    ) -> Option<AutoFixOperationSupport>
    where
        R: AutoFixOperationRunner + 'static,
    {
        let operation_key = support.operation_key;
        self.operations
            .insert(
                operation_key,
                AutoFixOperation {
                    support,
                    runner: Box::new(runner),
                },
            )
            .map(|operation| operation.support)
    }

    /// Returns true when no executable operations are registered.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Returns closure-free support metadata for registered operations.
    pub fn support_catalog(&self) -> AutoFixSupportCatalog {
        AutoFixSupportCatalog::from_registry(self)
    }

    fn operation(&self, operation_key: AutoFixOperationKey) -> Option<&AutoFixOperation> {
        self.operations.get(&operation_key)
    }
}

impl fmt::Debug for AutoFixRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AutoFixRegistry")
            .field("operation_count", &self.operations.len())
            .field(
                "operation_keys",
                &self.operations.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Fail-closed Auto-Fix service over an injected filesystem and registry.
///
/// [`Self::new`] registers zero operations, preserving empty production parity.
pub struct AutoFixService<'a> {
    filesystem: &'a dyn Filesystem,
    registry: AutoFixRegistry,
}

impl<'a> AutoFixService<'a> {
    /// Creates a production Auto-Fix service with an empty registry.
    pub fn new(filesystem: &'a dyn Filesystem) -> Self {
        Self {
            filesystem,
            registry: AutoFixRegistry::production(),
        }
    }

    /// Creates an Auto-Fix service with an injected registry.
    pub fn with_registry(filesystem: &'a dyn Filesystem, registry: AutoFixRegistry) -> Self {
        Self {
            filesystem,
            registry,
        }
    }

    /// Returns closure-free support metadata for controller/UI consumption.
    pub fn support_catalog(&self) -> AutoFixSupportCatalog {
        self.registry.support_catalog()
    }

    /// Plans an Auto-Fix operation for one selected scanner result.
    pub fn plan(
        &self,
        snapshot: &ScannerScanSnapshot,
        request: AutoFixPlanRequest,
    ) -> AutoFixPlanResult {
        let span = info_span!(
            "scanner.autofix.plan",
            scan_id = request.scan_id,
            result_index = request.result_index,
        );
        let _guard = span.enter();
        info!(
            event = "scanner-autofix-requested",
            scan_id = request.scan_id,
            result_index = request.result_index,
            "Scanner Auto-Fix plan requested"
        );

        let selection = match self.resolve_selection(
            snapshot,
            request.scan_id,
            request.result_index,
            &request.selection_identity,
            None,
        ) {
            Ok(selection) => selection,
            Err(rejection) => return rejected_plan(rejection),
        };

        let Some(operation_key) = selection.result.auto_fix_operation_key() else {
            return rejected_plan(reject(
                AutoFixRejectionReason::UnsupportedSolution,
                SAFE_UNAVAILABLE,
                Some(request.scan_id),
                Some(request.result_index),
                None,
                Some(request.selection_identity),
                None,
                Some("selected result has no retained typed Auto-Fix key".to_owned()),
            ));
        };

        let Some(operation) = self.registry.operation(operation_key) else {
            return rejected_plan(reject(
                AutoFixRejectionReason::NoRegisteredHandler,
                SAFE_UNAVAILABLE,
                Some(request.scan_id),
                Some(request.result_index),
                Some(operation_key),
                Some(selection.current_identity.clone()),
                None,
                Some(format!(
                    "no registered handler for {}",
                    operation_key.as_id()
                )),
            ));
        };

        if operation.support.requires_target_path && selection.result.absolute_path.is_none() {
            return rejected_plan(reject(
                AutoFixRejectionReason::MissingTargetPath,
                SAFE_TARGET_MISSING,
                Some(request.scan_id),
                Some(request.result_index),
                Some(operation_key),
                Some(selection.current_identity.clone()),
                None,
                Some("registered operation requires a target path".to_owned()),
            ));
        }

        let preview = AutoFixPlanPreview {
            operation_key,
            selection_identity: selection.current_identity.clone(),
            target_path: selection.result.absolute_path.clone(),
            safe_preview: operation.support.safe_preview.clone(),
            confirmation_required: operation.support.confirmation_required,
            confirmation_prompt: operation.support.confirmation_prompt.clone(),
            revalidation: AutoFixRevalidationPlan::required(selection.current_identity),
        };
        info!(
            event = "scanner-autofix-planned",
            scan_id = request.scan_id,
            result_index = request.result_index,
            operation_key = %operation_key.as_id(),
            confirmation_required = operation.support.confirmation_required,
            "Scanner Auto-Fix plan accepted"
        );
        AutoFixPlanResult::Planned(preview)
    }

    /// Executes an Auto-Fix request for one selected scanner result.
    pub fn execute(
        &self,
        snapshot: &ScannerScanSnapshot,
        result_index: usize,
        request: AutoFixRequest,
    ) -> AutoFixServiceResult {
        let span = info_span!(
            "scanner.autofix.execute",
            scan_id = ?request.scan_id,
            result_index,
            operation_key = %request.operation_key.as_id(),
        );
        let _guard = span.enter();
        info!(
            event = "scanner-autofix-execute-requested",
            scan_id = ?request.scan_id,
            result_index,
            operation_key = %request.operation_key.as_id(),
            "Scanner Auto-Fix execute requested"
        );

        let Some(scan_id) = request.scan_id else {
            return rejected_execution(reject(
                AutoFixRejectionReason::ScanMismatch,
                SAFE_SCAN_CHANGED,
                None,
                Some(result_index),
                Some(request.operation_key),
                Some(request.selection_identity),
                None,
                Some("request did not include a scan id".to_owned()),
            ));
        };

        let selection = match self.resolve_selection(
            snapshot,
            scan_id,
            result_index,
            &request.selection_identity,
            Some(request.operation_key),
        ) {
            Ok(selection) => selection,
            Err(rejection) => return rejected_execution(rejection),
        };

        let Some(selected_key) = selection.result.auto_fix_operation_key() else {
            return rejected_execution(reject(
                AutoFixRejectionReason::UnsupportedSolution,
                SAFE_UNAVAILABLE,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(selection.current_identity.clone()),
                None,
                Some("selected result has no retained typed Auto-Fix key".to_owned()),
            ));
        };

        if selected_key != request.operation_key {
            return rejected_execution(reject(
                AutoFixRejectionReason::OperationMismatch,
                SAFE_OPERATION_MISMATCH,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(selection.current_identity.clone()),
                None,
                Some(format!(
                    "request operation {} did not match selected result operation {}",
                    request.operation_key.as_id(),
                    selected_key.as_id()
                )),
            ));
        }

        let Some(operation) = self.registry.operation(request.operation_key) else {
            return rejected_execution(reject(
                AutoFixRejectionReason::NoRegisteredHandler,
                SAFE_UNAVAILABLE,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(selection.current_identity.clone()),
                None,
                Some(format!(
                    "no registered handler for {}",
                    request.operation_key.as_id()
                )),
            ));
        };

        if let Err(rejection) = validate_target(
            &request,
            selection.result,
            &operation.support,
            scan_id,
            result_index,
            &selection.current_identity,
        ) {
            return rejected_execution(rejection);
        }
        if let Err(rejection) =
            validate_revalidation(&request, scan_id, result_index, &selection.current_identity)
        {
            return rejected_execution(rejection);
        }
        if let Err(rejection) = validate_confirmation(
            &request,
            &operation.support,
            scan_id,
            result_index,
            &selection.current_identity,
        ) {
            return rejected_execution(rejection);
        }

        let context = AutoFixOperationContext {
            scan_id,
            result_index,
            operation_key: request.operation_key,
            selection_identity: &selection.current_identity,
            result: selection.result,
            target_path: request.target_path.as_deref(),
            filesystem: self.filesystem,
            confirmation: request.confirmation.as_ref(),
        };

        if let Err(failure) = operation.runner.validate_preconditions(&context) {
            return rejected_execution(reject(
                AutoFixRejectionReason::ValidationFailed,
                failure.safe_message,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(selection.current_identity.clone()),
                None,
                failure.diagnostic.or(Some(failure.details)),
            ));
        }

        info!(
            event = "scanner-autofix-scheduled",
            scan_id,
            result_index,
            operation_key = %request.operation_key.as_id(),
            "Scanner Auto-Fix operation scheduled"
        );
        let revalidation = request
            .revalidation
            .clone()
            .with_observed_identity(selection.current_identity.clone());

        match operation.runner.execute(&context) {
            Ok(success) => {
                let completion = AutoFixCompletion {
                    scan_id: Some(scan_id),
                    result_index: Some(result_index),
                    operation_key: request.operation_key,
                    selection_identity: selection.current_identity,
                    revalidation,
                    status: status(
                        AutoFixStatusKind::Fixed,
                        success.safe_summary.clone(),
                        success.diagnostic.clone(),
                    ),
                    detail: detail(success.safe_summary, success.details, success.diagnostic),
                };
                info!(
                    event = "scanner-autofix-completed",
                    scan_id,
                    result_index,
                    operation_key = %completion.operation_key.as_id(),
                    safe_message = %completion.status.safe_message,
                    "Scanner Auto-Fix operation completed"
                );
                AutoFixServiceResult::Completed(completion)
            }
            Err(failure) => {
                let completion = AutoFixCompletion {
                    scan_id: Some(scan_id),
                    result_index: Some(result_index),
                    operation_key: request.operation_key,
                    selection_identity: selection.current_identity,
                    revalidation,
                    status: status(
                        AutoFixStatusKind::Failed,
                        failure.safe_message.clone(),
                        failure.diagnostic.clone(),
                    ),
                    detail: detail(failure.safe_message, failure.details, failure.diagnostic),
                };
                warn!(
                    event = "scanner-autofix-failed",
                    scan_id,
                    result_index,
                    operation_key = %completion.operation_key.as_id(),
                    safe_message = %completion.status.safe_message,
                    "Scanner Auto-Fix operation failed"
                );
                AutoFixServiceResult::Completed(completion)
            }
        }
    }

    // Keep the owned domain rejection shape intact for callers/tests; this private path is not hot.
    #[allow(clippy::result_large_err)]
    fn resolve_selection<'snapshot>(
        &self,
        snapshot: &'snapshot ScannerScanSnapshot,
        scan_id: u64,
        result_index: usize,
        expected_identity: &AutoFixSelectionIdentity,
        operation_key: Option<AutoFixOperationKey>,
    ) -> Result<ResolvedSelection<'snapshot>, AutoFixRejection> {
        if snapshot.scan_id != scan_id {
            return Err(reject(
                AutoFixRejectionReason::ScanMismatch,
                SAFE_SCAN_CHANGED,
                Some(scan_id),
                Some(result_index),
                operation_key,
                Some(expected_identity.clone()),
                None,
                Some(format!(
                    "request scan id {scan_id} did not match snapshot scan id {}",
                    snapshot.scan_id
                )),
            ));
        }
        let Some(result) = snapshot.results.get(result_index) else {
            return Err(reject(
                AutoFixRejectionReason::ResultNotFound,
                SAFE_RESULT_MISSING,
                Some(scan_id),
                Some(result_index),
                operation_key,
                Some(expected_identity.clone()),
                None,
                Some(format!(
                    "result index {result_index} was outside {} result rows",
                    snapshot.results.len()
                )),
            ));
        };

        let current_identity = result.selection_identity();
        if &current_identity != expected_identity {
            return Err(reject(
                AutoFixRejectionReason::StaleSelection,
                SAFE_SCAN_CHANGED,
                Some(scan_id),
                Some(result_index),
                operation_key,
                Some(expected_identity.clone()),
                Some(current_identity),
                Some("selected result identity did not match current row identity".to_owned()),
            ));
        }

        Ok(ResolvedSelection {
            result,
            current_identity,
        })
    }
}

struct ResolvedSelection<'a> {
    result: &'a ScannerResult,
    current_identity: AutoFixSelectionIdentity,
}

// Keep owned rejection diagnostics available without changing the public service payload shape.
#[allow(clippy::result_large_err)]
fn validate_target(
    request: &AutoFixRequest,
    result: &ScannerResult,
    support: &AutoFixOperationSupport,
    scan_id: u64,
    result_index: usize,
    identity: &AutoFixSelectionIdentity,
) -> Result<(), AutoFixRejection> {
    if support.requires_target_path {
        let Some(result_target) = result.absolute_path.as_ref() else {
            return Err(reject(
                AutoFixRejectionReason::MissingTargetPath,
                SAFE_TARGET_MISSING,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(identity.clone()),
                None,
                Some("selected result has no target path".to_owned()),
            ));
        };
        let Some(request_target) = request.target_path.as_ref() else {
            return Err(reject(
                AutoFixRejectionReason::MissingTargetPath,
                SAFE_TARGET_MISSING,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(identity.clone()),
                None,
                Some("request has no target path".to_owned()),
            ));
        };
        if request_target != result_target {
            return Err(reject(
                AutoFixRejectionReason::TargetMismatch,
                SAFE_TARGET_MISMATCH,
                Some(scan_id),
                Some(result_index),
                Some(request.operation_key),
                Some(identity.clone()),
                None,
                Some(format!(
                    "request target {} did not match result target {}",
                    request_target.display(),
                    result_target.display()
                )),
            ));
        }
    } else if let Some(request_target) = request.target_path.as_ref()
        && result.absolute_path.as_ref() != Some(request_target)
    {
        return Err(reject(
            AutoFixRejectionReason::TargetMismatch,
            SAFE_TARGET_MISMATCH,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(identity.clone()),
            None,
            Some("request target path does not belong to selected result".to_owned()),
        ));
    }
    Ok(())
}

fn validate_revalidation(
    request: &AutoFixRequest,
    scan_id: u64,
    result_index: usize,
    current_identity: &AutoFixSelectionIdentity,
) -> Result<(), AutoFixRejection> {
    if request.revalidation.expected_identity != request.selection_identity {
        return Err(reject(
            AutoFixRejectionReason::ValidationFailed,
            SAFE_VALIDATION_FAILED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(request.selection_identity.clone()),
            Some(current_identity.clone()),
            Some("revalidation identity did not match request identity".to_owned()),
        ));
    }
    if !request.revalidation.required_before_mutation {
        return Err(reject(
            AutoFixRejectionReason::ValidationFailed,
            SAFE_VALIDATION_FAILED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(request.selection_identity.clone()),
            Some(current_identity.clone()),
            Some("request disabled pre-mutation revalidation".to_owned()),
        ));
    }
    if let Some(observed) = request.revalidation.observed_identity.as_ref()
        && observed != current_identity
    {
        return Err(reject(
            AutoFixRejectionReason::StaleSelection,
            SAFE_SCAN_CHANGED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(request.selection_identity.clone()),
            Some(current_identity.clone()),
            Some("observed identity did not match current row identity".to_owned()),
        ));
    }
    Ok(())
}

// Keep owned rejection diagnostics available without changing the public service payload shape.
#[allow(clippy::result_large_err)]
fn validate_confirmation(
    request: &AutoFixRequest,
    support: &AutoFixOperationSupport,
    scan_id: u64,
    result_index: usize,
    current_identity: &AutoFixSelectionIdentity,
) -> Result<(), AutoFixRejection> {
    if support.confirmation_required && request.confirmation.is_none() {
        return Err(reject(
            AutoFixRejectionReason::ConfirmationRequired,
            SAFE_CONFIRMATION_REQUIRED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(current_identity.clone()),
            None,
            Some("registered operation requires confirmation".to_owned()),
        ));
    }
    let Some(confirmation) = request.confirmation.as_ref() else {
        return Ok(());
    };
    if confirmation.operation_key != request.operation_key {
        return Err(reject(
            AutoFixRejectionReason::OperationMismatch,
            SAFE_OPERATION_MISMATCH,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(current_identity.clone()),
            None,
            Some("confirmation operation did not match request operation".to_owned()),
        ));
    }
    if confirmation.selection_identity != request.selection_identity {
        return Err(reject(
            AutoFixRejectionReason::StaleSelection,
            SAFE_SCAN_CHANGED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(request.selection_identity.clone()),
            Some(current_identity.clone()),
            Some("confirmation identity did not match request identity".to_owned()),
        ));
    }
    if !confirmation.accepted {
        return Err(reject(
            AutoFixRejectionReason::ConfirmationDeclined,
            SAFE_CONFIRMATION_DECLINED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(current_identity.clone()),
            None,
            Some("confirmation was declined".to_owned()),
        ));
    }
    if confirmation.requires_pre_mutation_revalidation
        && !request.revalidation.required_before_mutation
    {
        return Err(reject(
            AutoFixRejectionReason::ValidationFailed,
            SAFE_VALIDATION_FAILED,
            Some(scan_id),
            Some(result_index),
            Some(request.operation_key),
            Some(current_identity.clone()),
            None,
            Some("confirmation required pre-mutation revalidation".to_owned()),
        ));
    }
    Ok(())
}

fn status(
    kind: AutoFixStatusKind,
    safe_message: String,
    diagnostic: Option<String>,
) -> AutoFixStatus {
    let status = AutoFixStatus::new(kind, safe_message);
    match diagnostic {
        Some(diagnostic) => status.with_diagnostic(diagnostic),
        None => status,
    }
}

fn detail(
    safe_summary: String,
    details: String,
    diagnostic: Option<String>,
) -> AutoFixResultDetail {
    let detail = AutoFixResultDetail::new(safe_summary, details);
    match diagnostic {
        Some(diagnostic) => detail.with_diagnostic(diagnostic),
        None => detail,
    }
}

// Centralizes context-rich rejection construction for observability and tests.
#[allow(clippy::too_many_arguments)]
fn reject(
    reason: AutoFixRejectionReason,
    safe_message: impl Into<String>,
    scan_id: Option<u64>,
    result_index: Option<usize>,
    operation_key: Option<AutoFixOperationKey>,
    selection_identity: Option<AutoFixSelectionIdentity>,
    observed_identity: Option<AutoFixSelectionIdentity>,
    diagnostic: Option<String>,
) -> AutoFixRejection {
    let mut rejection = AutoFixRejection::new(reason, safe_message);
    rejection.scan_id = scan_id;
    rejection.result_index = result_index;
    rejection.operation_key = operation_key;
    rejection.selection_identity = selection_identity;
    rejection.observed_identity = observed_identity;
    rejection.diagnostic = diagnostic;
    rejection
}

fn rejected_plan(rejection: AutoFixRejection) -> AutoFixPlanResult {
    log_rejection(&rejection);
    AutoFixPlanResult::Rejected(rejection)
}

fn rejected_execution(rejection: AutoFixRejection) -> AutoFixServiceResult {
    log_rejection(&rejection);
    AutoFixServiceResult::Rejected(rejection)
}

fn log_rejection(rejection: &AutoFixRejection) {
    let operation_id = rejection
        .operation_key
        .map(AutoFixOperationKey::as_id)
        .unwrap_or("unknown");
    warn!(
        event = "scanner-autofix-rejected",
        scan_id = ?rejection.scan_id,
        result_index = ?rejection.result_index,
        operation_key = %operation_id,
        rejection_reason = ?rejection.reason,
        safe_message = %rejection.safe_message,
        "Scanner Auto-Fix request rejected"
    );
    if matches!(rejection.reason, AutoFixRejectionReason::StaleSelection) {
        warn!(
            event = "scanner-autofix-stale",
            scan_id = ?rejection.scan_id,
            result_index = ?rejection.result_index,
            operation_key = %operation_id,
            "Scanner Auto-Fix request rejected as stale"
        );
    }
}

#[cfg(test)]
mod scanner_autofix_service {
    use std::{
        collections::BTreeSet,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use crate::{
        domain::{
            autofix::AutoFixConfirmation,
            scanner::{ScannerProblemType, ScannerResult, ScannerScanSnapshot},
        },
        platform::{
            PlatformError, PlatformErrorKind, PlatformOperation, PlatformResult,
            filesystem::{DirectoryEntry, FileMetadata, FileType},
        },
    };

    use super::*;

    #[derive(Debug, Default)]
    struct FakeFilesystem {
        files: BTreeSet<PathBuf>,
    }

    impl FakeFilesystem {
        fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
            self.files.insert(path.into());
            self
        }
    }

    impl Filesystem for FakeFilesystem {
        fn metadata(&self, path: &Path) -> PlatformResult<FileMetadata> {
            if self.files.contains(path) {
                Ok(FileMetadata {
                    file_type: FileType::File,
                    len: 1,
                })
            } else {
                Err(PlatformError::new(
                    PlatformOperation::ReadMetadata,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "Filesystem metadata read target was not found.",
                ))
            }
        }

        fn read_bytes(&self, path: &Path) -> PlatformResult<Vec<u8>> {
            if self.files.contains(path) {
                Ok(vec![1])
            } else {
                Err(PlatformError::new(
                    PlatformOperation::ReadFile,
                    path.display().to_string(),
                    PlatformErrorKind::NotFound,
                    "File read target was not found.",
                ))
            }
        }

        fn read_to_string(&self, path: &Path) -> PlatformResult<String> {
            self.read_bytes(path).map(|_| "fake".to_owned())
        }

        fn read_dir(&self, _path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            Ok(Vec::new())
        }

        fn walk_dir(&self, _path: &Path) -> PlatformResult<Vec<DirectoryEntry>> {
            Ok(Vec::new())
        }
    }

    #[derive(Clone, Copy)]
    enum FakePrecondition {
        Pass,
        RequireExistingTarget,
    }

    #[derive(Clone, Copy)]
    enum FakeExecution {
        Succeed,
        Fail,
    }

    struct FakeOperation {
        precondition: FakePrecondition,
        execution: FakeExecution,
        precondition_calls: Arc<AtomicUsize>,
        execute_calls: Arc<AtomicUsize>,
    }

    impl FakeOperation {
        fn new(
            precondition: FakePrecondition,
            execution: FakeExecution,
        ) -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            let precondition_calls = Arc::new(AtomicUsize::new(0));
            let execute_calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    precondition,
                    execution,
                    precondition_calls: Arc::clone(&precondition_calls),
                    execute_calls: Arc::clone(&execute_calls),
                },
                precondition_calls,
                execute_calls,
            )
        }
    }

    impl AutoFixOperationRunner for FakeOperation {
        fn validate_preconditions(
            &self,
            context: &AutoFixOperationContext<'_>,
        ) -> Result<(), AutoFixOperationFailure> {
            self.precondition_calls.fetch_add(1, Ordering::SeqCst);
            match self.precondition {
                FakePrecondition::Pass => Ok(()),
                FakePrecondition::RequireExistingTarget => {
                    let Some(target_path) = context.target_path else {
                        return Err(AutoFixOperationFailure::new(
                            SAFE_VALIDATION_FAILED,
                            "The fake target was missing before mutation.",
                        )
                        .with_diagnostic("missing fake target"));
                    };
                    context
                        .filesystem
                        .metadata(target_path)
                        .map(|_| ())
                        .map_err(|error| {
                            AutoFixOperationFailure::new(
                                SAFE_VALIDATION_FAILED,
                                "The fake target did not pass filesystem revalidation.",
                            )
                            .with_diagnostic(format!(
                                "{:?}: {}",
                                error.kind,
                                error.user_message()
                            ))
                        })
                }
            }
        }

        fn execute(
            &self,
            context: &AutoFixOperationContext<'_>,
        ) -> Result<AutoFixOperationSuccess, AutoFixOperationFailure> {
            self.execute_calls.fetch_add(1, Ordering::SeqCst);
            match self.execution {
                FakeExecution::Succeed => Ok(AutoFixOperationSuccess::new(
                    "Fixed fake scanner result.",
                    format!(
                        "Fixed {} at row {}.",
                        context.operation_key.as_id(),
                        context.result_index
                    ),
                )
                .with_diagnostic("fake success diagnostic")),
                FakeExecution::Fail => Err(AutoFixOperationFailure::new(
                    "Auto-Fix could not complete this operation.",
                    "The fake operation reported a controlled failure.",
                )
                .with_diagnostic("fake operation failure")),
            }
        }
    }

    fn support(operation_key: AutoFixOperationKey) -> AutoFixOperationSupport {
        AutoFixOperationSupport::new(operation_key, "Fake Fix", "Fake preview")
    }

    fn service_with_operation(
        filesystem: &dyn Filesystem,
        support: AutoFixOperationSupport,
        operation: FakeOperation,
    ) -> AutoFixService<'_> {
        let mut registry = AutoFixRegistry::empty();
        registry.register(support, operation);
        AutoFixService::with_registry(filesystem, registry)
    }

    fn pathless_result(solution_kind: ScannerSolutionKind) -> ScannerResult {
        ScannerResult::simple(
            ScannerProblemType::JunkFile,
            "fake-row",
            "Fake problem summary.",
            None,
        )
        .with_solution_kind(solution_kind)
    }

    fn path_result(solution_kind: ScannerSolutionKind, path: &str) -> ScannerResult {
        ScannerResult::with_path(
            ScannerProblemType::JunkFile,
            PathBuf::from(path),
            PathBuf::from(path),
            "Fake problem summary.",
            None,
        )
        .with_solution_kind(solution_kind)
    }

    fn display_only_result() -> ScannerResult {
        ScannerResult::simple(
            ScannerProblemType::JunkFile,
            "fake-row",
            "Fake problem summary.",
            Some(
                ScannerSolutionKind::DeleteFile
                    .as_reference_text()
                    .to_owned(),
            ),
        )
    }

    fn snapshot(result: ScannerResult) -> ScannerScanSnapshot {
        ScannerScanSnapshot::from_results(42, vec![result], "Scanner completed with 1 results.")
    }

    fn plan_request(snapshot: &ScannerScanSnapshot) -> AutoFixPlanRequest {
        AutoFixPlanRequest::new(42, 0, snapshot.results[0].selection_identity())
    }

    fn request_from_preview(preview: AutoFixPlanPreview) -> AutoFixRequest {
        AutoFixRequest {
            scan_id: Some(42),
            operation_key: preview.operation_key,
            selection_identity: preview.selection_identity.clone(),
            target_path: preview.target_path,
            confirmation: None,
            revalidation: preview.revalidation,
        }
    }

    fn confirmation_for(preview: &AutoFixPlanPreview, accepted: bool) -> AutoFixConfirmation {
        AutoFixConfirmation {
            operation_key: preview.operation_key,
            selection_identity: preview.selection_identity.clone(),
            accepted,
            prompt: preview.confirmation_prompt.clone(),
            confirmation_token: None,
            requires_pre_mutation_revalidation: true,
        }
    }

    #[test]
    fn scanner_autofix_service_production_registry_is_empty_and_rejects_typed_solution() {
        let filesystem = FakeFilesystem::default();
        let service = AutoFixService::new(&filesystem);
        let snapshot = snapshot(pathless_result(ScannerSolutionKind::DeleteFile));

        assert!(service.support_catalog().is_empty());
        assert_eq!(
            service
                .support_catalog()
                .support_for_result(&snapshot.results[0]),
            None
        );

        let AutoFixPlanResult::Rejected(rejection) =
            service.plan(&snapshot, plan_request(&snapshot))
        else {
            panic!("empty production registry must reject typed Auto-Fix solutions");
        };
        assert_eq!(
            rejection.reason,
            AutoFixRejectionReason::NoRegisteredHandler
        );
        assert_eq!(rejection.scan_id, Some(42));
        assert_eq!(rejection.result_index, Some(0));
        assert_eq!(
            rejection.operation_key,
            Some(AutoFixOperationKey::DeleteFile)
        );
        assert_eq!(rejection.safe_message, SAFE_UNAVAILABLE);
    }

    #[test]
    fn scanner_autofix_service_catalog_uses_typed_solution_and_rejects_stale_inputs() {
        let filesystem = FakeFilesystem::default();
        let (operation, _precondition_calls, execute_calls) =
            FakeOperation::new(FakePrecondition::Pass, FakeExecution::Succeed);
        let service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile),
            operation,
        );
        let supported = snapshot(pathless_result(ScannerSolutionKind::DeleteFile));
        let display_only = snapshot(display_only_result());
        let identity = supported.results[0].selection_identity();
        let catalog = service.support_catalog();

        assert_eq!(catalog.len(), 1);
        assert!(catalog.support_for_result(&supported.results[0]).is_some());
        assert!(
            catalog
                .support_for_result(&display_only.results[0])
                .is_none()
        );

        let AutoFixPlanResult::Rejected(unsupported) =
            service.plan(&display_only, plan_request(&display_only))
        else {
            panic!("display-only solution text must not create Auto-Fix eligibility");
        };
        assert_eq!(
            unsupported.reason,
            AutoFixRejectionReason::UnsupportedSolution
        );

        let AutoFixPlanResult::Rejected(scan_mismatch) =
            service.plan(&supported, AutoFixPlanRequest::new(99, 0, identity.clone()))
        else {
            panic!("wrong scan id must reject");
        };
        assert_eq!(scan_mismatch.reason, AutoFixRejectionReason::ScanMismatch);

        let AutoFixPlanResult::Rejected(missing_index) =
            service.plan(&supported, AutoFixPlanRequest::new(42, 2, identity.clone()))
        else {
            panic!("unknown row index must reject");
        };
        assert_eq!(missing_index.reason, AutoFixRejectionReason::ResultNotFound);

        let AutoFixPlanResult::Rejected(stale) = service.plan(
            &supported,
            AutoFixPlanRequest::new(
                42,
                0,
                AutoFixSelectionIdentity::from_fingerprint("scanner-result:v1:stale"),
            ),
        ) else {
            panic!("stale selected identity must reject");
        };
        assert_eq!(stale.reason, AutoFixRejectionReason::StaleSelection);
        assert_eq!(execute_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn scanner_autofix_service_fake_success_and_failure_return_owned_completion_payloads() {
        let filesystem = FakeFilesystem::default();
        let (success_op, success_preconditions, success_executes) =
            FakeOperation::new(FakePrecondition::Pass, FakeExecution::Succeed);
        let success_service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile),
            success_op,
        );
        let success_snapshot = snapshot(pathless_result(ScannerSolutionKind::DeleteFile));
        let AutoFixPlanResult::Planned(success_preview) =
            success_service.plan(&success_snapshot, plan_request(&success_snapshot))
        else {
            panic!("registered fake operation should plan");
        };

        let AutoFixServiceResult::Completed(success) =
            success_service.execute(&success_snapshot, 0, request_from_preview(success_preview))
        else {
            panic!("valid fake request should complete");
        };
        assert_eq!(success.status.kind, AutoFixStatusKind::Fixed);
        assert_eq!(success.status.safe_message, "Fixed fake scanner result.");
        assert_eq!(success.scan_id, Some(42));
        assert_eq!(success.result_index, Some(0));
        assert_eq!(success_preconditions.load(Ordering::SeqCst), 1);
        assert_eq!(success_executes.load(Ordering::SeqCst), 1);

        let (fail_op, fail_preconditions, fail_executes) =
            FakeOperation::new(FakePrecondition::Pass, FakeExecution::Fail);
        let fail_service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile),
            fail_op,
        );
        let fail_snapshot = snapshot(pathless_result(ScannerSolutionKind::DeleteFile));
        let AutoFixPlanResult::Planned(fail_preview) =
            fail_service.plan(&fail_snapshot, plan_request(&fail_snapshot))
        else {
            panic!("registered fake operation should plan");
        };

        let AutoFixServiceResult::Completed(failure) =
            fail_service.execute(&fail_snapshot, 0, request_from_preview(fail_preview))
        else {
            panic!("operation failures are completed payloads, not pre-mutation rejections");
        };
        assert_eq!(failure.status.kind, AutoFixStatusKind::Failed);
        assert_eq!(
            failure.status.safe_message,
            "Auto-Fix could not complete this operation."
        );
        assert_eq!(
            failure.detail.details,
            "The fake operation reported a controlled failure."
        );
        assert_eq!(
            failure.detail.diagnostic.as_deref(),
            Some("fake operation failure")
        );
        assert_eq!(fail_preconditions.load(Ordering::SeqCst), 1);
        assert_eq!(fail_executes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn scanner_autofix_service_rejects_missing_target_and_confirmation_before_execute() {
        let filesystem = FakeFilesystem::default().with_file("Data/junk.txt");
        let (pathless_op, _pathless_preconditions, pathless_executes) =
            FakeOperation::new(FakePrecondition::Pass, FakeExecution::Succeed);
        let pathless_service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile).with_required_target_path(),
            pathless_op,
        );
        let pathless_snapshot = snapshot(pathless_result(ScannerSolutionKind::DeleteFile));

        let AutoFixPlanResult::Rejected(missing_target) =
            pathless_service.plan(&pathless_snapshot, plan_request(&pathless_snapshot))
        else {
            panic!("target-required operation must reject pathless results");
        };
        assert_eq!(
            missing_target.reason,
            AutoFixRejectionReason::MissingTargetPath
        );
        assert_eq!(pathless_executes.load(Ordering::SeqCst), 0);

        let (confirm_op, _confirm_preconditions, confirm_executes) =
            FakeOperation::new(FakePrecondition::Pass, FakeExecution::Succeed);
        let confirm_service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile)
                .with_required_target_path()
                .with_confirmation("Delete this fake file?"),
            confirm_op,
        );
        let confirm_snapshot = snapshot(path_result(
            ScannerSolutionKind::DeleteFile,
            "Data/junk.txt",
        ));
        let AutoFixPlanResult::Planned(preview) =
            confirm_service.plan(&confirm_snapshot, plan_request(&confirm_snapshot))
        else {
            panic!("target-backed operation should plan before confirmation");
        };

        let AutoFixServiceResult::Rejected(required) =
            confirm_service.execute(&confirm_snapshot, 0, request_from_preview(preview.clone()))
        else {
            panic!("missing confirmation must reject before execute");
        };
        assert_eq!(
            required.reason,
            AutoFixRejectionReason::ConfirmationRequired
        );
        assert_eq!(confirm_executes.load(Ordering::SeqCst), 0);

        let mut declined_request = request_from_preview(preview.clone());
        declined_request.confirmation = Some(confirmation_for(&preview, false));
        let AutoFixServiceResult::Rejected(declined) =
            confirm_service.execute(&confirm_snapshot, 0, declined_request)
        else {
            panic!("declined confirmation must reject before execute");
        };
        assert_eq!(
            declined.reason,
            AutoFixRejectionReason::ConfirmationDeclined
        );
        assert_eq!(confirm_executes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn scanner_autofix_service_rejects_failed_precondition_before_execute() {
        let filesystem = FakeFilesystem::default();
        let (operation, preconditions, executes) = FakeOperation::new(
            FakePrecondition::RequireExistingTarget,
            FakeExecution::Succeed,
        );
        let service = service_with_operation(
            &filesystem,
            support(AutoFixOperationKey::DeleteFile).with_required_target_path(),
            operation,
        );
        let snapshot = snapshot(path_result(
            ScannerSolutionKind::DeleteFile,
            "Data/missing.txt",
        ));
        let AutoFixPlanResult::Planned(preview) = service.plan(&snapshot, plan_request(&snapshot))
        else {
            panic!("precondition checks happen during execute, not plan");
        };

        let AutoFixServiceResult::Rejected(rejection) =
            service.execute(&snapshot, 0, request_from_preview(preview))
        else {
            panic!("failed precondition must reject before execute");
        };
        assert_eq!(rejection.reason, AutoFixRejectionReason::ValidationFailed);
        assert_eq!(rejection.safe_message, SAFE_VALIDATION_FAILED);
        assert!(
            rejection
                .diagnostic
                .as_deref()
                .is_some_and(|diagnostic| diagnostic.contains("NotFound"))
        );
        assert_eq!(preconditions.load(Ordering::SeqCst), 1);
        assert_eq!(executes.load(Ordering::SeqCst), 0);
    }
}
