//! Injectable registry adapter contracts.
//!
//! The Python reference reads Windows registry values for Fallout 4 and MO2
//! discovery. This module keeps that OS access behind a fakeable trait and
//! returns explicit unsupported-platform failures on non-Windows hosts.

use std::fmt;

use crate::platform::{PlatformError, PlatformOperation, PlatformResult};

/// Registry hives used by CMT discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegistryHive {
    /// `HKEY_CURRENT_USER`.
    CurrentUser,
    /// `HKEY_LOCAL_MACHINE`.
    LocalMachine,
}

impl RegistryHive {
    /// Returns the conventional Windows hive prefix.
    pub const fn as_windows_prefix(self) -> &'static str {
        match self {
            Self::CurrentUser => "HKEY_CURRENT_USER",
            Self::LocalMachine => "HKEY_LOCAL_MACHINE",
        }
    }
}

impl fmt::Display for RegistryHive {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_windows_prefix())
    }
}

/// Registry value request used for typed diagnostics and fake keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegistryValueRequest {
    /// Registry hive to query.
    pub hive: RegistryHive,
    /// Subkey path below the hive.
    pub subkey: String,
    /// Value name within the subkey.
    pub value_name: String,
}

impl RegistryValueRequest {
    /// Creates a registry string-value request.
    pub fn new(
        hive: RegistryHive,
        subkey: impl Into<String>,
        value_name: impl Into<String>,
    ) -> Self {
        Self {
            hive,
            subkey: subkey.into(),
            value_name: value_name.into(),
        }
    }

    /// Returns a display target suitable for diagnostics and action results.
    pub fn target(&self) -> String {
        format!(
            "{}\\{}:{}",
            self.hive.as_windows_prefix(),
            self.subkey,
            self.value_name
        )
    }
}

/// Fakeable registry reader for discovery code.
pub trait RegistryReader {
    /// Reads a string value, returning `Ok(None)` when the value is absent.
    fn read_string_value(&self, request: &RegistryValueRequest) -> PlatformResult<Option<String>>;
}

/// Production registry adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealRegistry;

impl RealRegistry {
    /// Creates the production registry adapter without querying the registry.
    pub const fn new() -> Self {
        Self
    }
}

impl RegistryReader for RealRegistry {
    fn read_string_value(&self, request: &RegistryValueRequest) -> PlatformResult<Option<String>> {
        read_real_registry_string(request)
    }
}

#[cfg(not(windows))]
fn read_real_registry_string(request: &RegistryValueRequest) -> PlatformResult<Option<String>> {
    Err(PlatformError::unsupported(
        PlatformOperation::ReadRegistry,
        request.target(),
    ))
}

#[cfg(windows)]
fn read_real_registry_string(request: &RegistryValueRequest) -> PlatformResult<Option<String>> {
    use windows_registry::{CURRENT_USER, LOCAL_MACHINE};

    let hive = match request.hive {
        RegistryHive::CurrentUser => CURRENT_USER,
        RegistryHive::LocalMachine => LOCAL_MACHINE,
    };

    let key = match hive.open(&request.subkey) {
        Ok(key) => key,
        Err(error) if registry_error_message_is_absent(&error.message()) => return Ok(None),
        Err(error) => {
            return Err(PlatformError::command_failed(
                PlatformOperation::ReadRegistry,
                request.target(),
                error.message(),
            ));
        }
    };

    match key.get_string(&request.value_name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(error) if registry_error_message_is_absent(&error.message()) => Ok(None),
        Err(error) => Err(PlatformError::command_failed(
            PlatformOperation::ReadRegistry,
            request.target(),
            error.message(),
        )),
    }
}

#[cfg(windows)]
fn registry_error_message_is_absent(message: &str) -> bool {
    // The reference helper returns `None` for missing registry keys/values. The
    // windows-registry crate exposes rich Windows errors; message matching is
    // used only to distinguish absence from other typed adapter failures here.
    let message = message.to_ascii_lowercase();
    message.contains("cannot find")
        || message.contains("not find")
        || message.contains("not found")
        || message.contains("does not exist")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::platform::{PlatformErrorKind, PlatformOperation};

    use super::*;

    #[derive(Debug, Default)]
    struct FakeRegistry {
        values: BTreeMap<RegistryValueRequest, PlatformResult<Option<String>>>,
    }

    impl FakeRegistry {
        fn with_value(mut self, request: RegistryValueRequest, value: Option<&str>) -> Self {
            self.values
                .insert(request, Ok(value.map(std::string::ToString::to_string)));
            self
        }

        fn with_failure(mut self, request: RegistryValueRequest, kind: PlatformErrorKind) -> Self {
            let target = request.target();
            self.values.insert(
                request,
                Err(PlatformError::new(
                    PlatformOperation::ReadRegistry,
                    target,
                    kind,
                    "Registry access failed.",
                )),
            );
            self
        }
    }

    impl RegistryReader for FakeRegistry {
        fn read_string_value(
            &self,
            request: &RegistryValueRequest,
        ) -> PlatformResult<Option<String>> {
            self.values.get(request).cloned().unwrap_or(Ok(None))
        }
    }

    #[test]
    fn fake_registry_returns_string_values_without_real_registry() {
        let request = RegistryValueRequest::new(
            RegistryHive::LocalMachine,
            r"SOFTWARE\WOW6432Node\Bethesda Softworks\Fallout4",
            "Installed Path",
        );
        let registry =
            FakeRegistry::default().with_value(request.clone(), Some(r"C:\Games\Fallout 4"));

        assert_eq!(
            registry
                .read_string_value(&request)
                .expect("fake registry should return value"),
            Some(r"C:\Games\Fallout 4".to_owned())
        );
    }

    #[test]
    fn fake_registry_surfaces_typed_failures() {
        let request = RegistryValueRequest::new(
            RegistryHive::CurrentUser,
            r"Software\Mod Organizer Team\Mod Organizer",
            "CurrentInstance",
        );
        let registry = FakeRegistry::default()
            .with_failure(request.clone(), PlatformErrorKind::PermissionDenied);

        let error = registry
            .read_string_value(&request)
            .expect_err("fake registry failure should be typed");

        assert_eq!(error.operation, PlatformOperation::ReadRegistry);
        assert_eq!(error.kind, PlatformErrorKind::PermissionDenied);
        assert_eq!(error.user_message(), "Registry access failed.");
    }

    #[cfg(not(windows))]
    #[test]
    fn real_registry_is_explicitly_unsupported_off_windows() {
        let request = RegistryValueRequest::new(
            RegistryHive::LocalMachine,
            r"SOFTWARE\WOW6432Node\Bethesda Softworks\Fallout4",
            "Installed Path",
        );

        let error = RealRegistry::new()
            .read_string_value(&request)
            .expect_err("non-Windows registry should be unsupported");

        assert_eq!(error.kind, PlatformErrorKind::UnsupportedPlatform);
        assert_eq!(error.operation, PlatformOperation::ReadRegistry);
        assert_eq!(
            error.user_message(),
            "Registry access is not supported on this platform."
        );
    }
}
