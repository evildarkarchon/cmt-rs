//! Injectable process, executable-version, and system-metadata adapters.
//!
//! The reference app inspects parent processes to detect MO2/Vortex, reads file
//! version metadata from executables, and later displays PC specs. This module
//! exposes those operations through fakeable traits and explicit typed failures.

use std::path::{Path, PathBuf};

use crate::{
    domain::discovery::SemanticVersion,
    platform::{PlatformError, PlatformOperation, PlatformResult},
};

/// Single process record from a platform process table snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessInfo {
    /// Operating-system process identifier.
    pub pid: u32,
    /// Parent process identifier when available.
    pub parent_pid: Option<u32>,
    /// Process executable name, such as `ModOrganizer.exe`.
    pub name: String,
    /// Full executable path when the platform can report it.
    pub executable_path: Option<PathBuf>,
}

impl ProcessInfo {
    /// Creates a typed process record.
    pub fn new(
        pid: u32,
        parent_pid: Option<u32>,
        name: impl Into<String>,
        executable_path: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self {
            pid,
            parent_pid,
            name: name.into(),
            executable_path: executable_path.map(Into::into),
        }
    }
}

/// Executable or DLL version metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionMetadata {
    /// Three-part semantic version used by domain manager contracts.
    pub semantic: SemanticVersion,
    /// Raw platform version string when one was available.
    pub raw: Option<String>,
}

impl VersionMetadata {
    /// Creates version metadata from a semantic version and optional raw string.
    pub fn new(semantic: SemanticVersion, raw: Option<impl Into<String>>) -> Self {
        Self {
            semantic,
            raw: raw.map(Into::into),
        }
    }
}

/// PC specs and operating-system metadata collected behind the platform seam.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemMetadata {
    /// Operating-system family or caption.
    pub os_name: String,
    /// Operating-system version/build when available.
    pub os_version: Option<String>,
    /// CPU architecture reported by Rust for the running binary.
    pub architecture: String,
    /// CPU model/brand when available.
    pub cpu_brand: Option<String>,
    /// Physical memory in bytes when available.
    pub physical_memory_bytes: Option<u64>,
    /// Logical CPU count when available.
    pub logical_cpu_count: Option<usize>,
}

impl SystemMetadata {
    /// Creates system metadata from already-collected values.
    pub fn new(
        os_name: impl Into<String>,
        os_version: Option<impl Into<String>>,
        architecture: impl Into<String>,
        cpu_brand: Option<impl Into<String>>,
        physical_memory_bytes: Option<u64>,
        logical_cpu_count: Option<usize>,
    ) -> Self {
        Self {
            os_name: os_name.into(),
            os_version: os_version.map(Into::into),
            architecture: architecture.into(),
            cpu_brand: cpu_brand.map(Into::into),
            physical_memory_bytes,
            logical_cpu_count,
        }
    }
}

/// Fakeable process and metadata inspection boundary.
pub trait ProcessInspector {
    /// Returns a point-in-time process table snapshot.
    fn list_processes(&self) -> PlatformResult<Vec<ProcessInfo>>;

    /// Reads executable or DLL version metadata, returning `Ok(None)` when absent.
    fn file_version(&self, path: &Path) -> PlatformResult<Option<VersionMetadata>>;

    /// Reads PC specs and operating-system metadata.
    fn system_metadata(&self) -> PlatformResult<SystemMetadata>;
}

/// Production process inspector.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealProcessInspector;

impl RealProcessInspector {
    /// Creates the production process inspector without querying the OS.
    pub const fn new() -> Self {
        Self
    }
}

impl ProcessInspector for RealProcessInspector {
    fn list_processes(&self) -> PlatformResult<Vec<ProcessInfo>> {
        list_real_processes()
    }

    fn file_version(&self, path: &Path) -> PlatformResult<Option<VersionMetadata>> {
        read_real_file_version(path)
    }

    fn system_metadata(&self) -> PlatformResult<SystemMetadata> {
        read_real_system_metadata()
    }
}

#[cfg(not(windows))]
fn list_real_processes() -> PlatformResult<Vec<ProcessInfo>> {
    Err(PlatformError::unsupported(
        PlatformOperation::ListProcesses,
        "process table",
    ))
}

#[cfg(not(windows))]
fn read_real_file_version(path: &Path) -> PlatformResult<Option<VersionMetadata>> {
    Err(PlatformError::unsupported(
        PlatformOperation::ReadVersionMetadata,
        path.display().to_string(),
    ))
}

#[cfg(not(windows))]
fn read_real_system_metadata() -> PlatformResult<SystemMetadata> {
    Err(PlatformError::unsupported(
        PlatformOperation::ReadSystemMetadata,
        "system metadata",
    ))
}

#[cfg(windows)]
fn list_real_processes() -> PlatformResult<Vec<ProcessInfo>> {
    let system = sysinfo::System::new_all();
    let mut processes = system
        .processes()
        .values()
        .map(|process| {
            ProcessInfo::new(
                process.pid().as_u32(),
                process.parent().map(|pid| pid.as_u32()),
                process.name().to_string_lossy(),
                process.exe().map(Path::to_path_buf),
            )
        })
        .collect::<Vec<_>>();

    processes.sort_by(|left, right| {
        left.pid
            .cmp(&right.pid)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(processes)
}

#[cfg(windows)]
fn read_real_system_metadata() -> PlatformResult<SystemMetadata> {
    let system = sysinfo::System::new_all();
    let os_name = sysinfo::System::long_os_version()
        .or_else(sysinfo::System::name)
        .unwrap_or_else(|| "Windows".to_owned());
    let cpu_brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_owned())
        .filter(|brand| !brand.is_empty());
    let logical_cpu_count = Some(system.cpus().len()).filter(|count| *count > 0);

    Ok(SystemMetadata::new(
        os_name,
        sysinfo::System::os_version(),
        std::env::consts::ARCH,
        cpu_brand,
        Some(system.total_memory()),
        logical_cpu_count,
    ))
}

#[cfg(windows)]
fn read_real_file_version(path: &Path) -> PlatformResult<Option<VersionMetadata>> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr};

    use windows::{
        Win32::Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
        },
        core::PCWSTR,
    };

    let target = path.display().to_string();
    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut ignored_handle = 0u32;
    let size =
        unsafe { GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), Some(&mut ignored_handle)) };
    if size == 0 {
        return Ok(None);
    }

    let mut bytes = vec![0u8; size as usize];
    unsafe {
        GetFileVersionInfoW(
            PCWSTR(wide_path.as_ptr()),
            Some(0),
            size,
            bytes.as_mut_ptr().cast::<c_void>(),
        )
    }
    .map_err(|error| {
        PlatformError::command_failed(
            PlatformOperation::ReadVersionMetadata,
            target.clone(),
            error.message(),
        )
    })?;

    let root_block = "\\"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut version_ptr: *mut c_void = ptr::null_mut();
    let mut version_len = 0u32;
    let queried = unsafe {
        VerQueryValueW(
            bytes.as_ptr().cast::<c_void>(),
            PCWSTR(root_block.as_ptr()),
            &mut version_ptr,
            &mut version_len,
        )
    };
    if !queried.as_bool() || version_ptr.is_null() {
        return Ok(None);
    }

    let fixed = unsafe { *(version_ptr.cast::<VS_FIXEDFILEINFO>()) };
    let major = ((fixed.dwFileVersionMS >> 16) & 0xffff) as u64;
    let minor = (fixed.dwFileVersionMS & 0xffff) as u64;
    let patch = ((fixed.dwFileVersionLS >> 16) & 0xffff) as u64;
    let private = fixed.dwFileVersionLS & 0xffff;
    let raw = format!("{major}.{minor}.{patch}.{private}");

    Ok(Some(VersionMetadata::new(
        SemanticVersion::new(major, minor, patch),
        Some(raw),
    )))
}

#[cfg(test)]
mod tests {
    use crate::platform::{PlatformErrorKind, PlatformOperation};

    use super::*;

    #[derive(Debug, Clone)]
    struct FakeProcessInspector {
        processes: PlatformResult<Vec<ProcessInfo>>,
        version: PlatformResult<Option<VersionMetadata>>,
        system: PlatformResult<SystemMetadata>,
    }

    impl Default for FakeProcessInspector {
        fn default() -> Self {
            Self {
                processes: Ok(Vec::new()),
                version: Ok(None),
                system: Ok(SystemMetadata::new(
                    "Windows",
                    Some("10.0"),
                    "x86_64",
                    Some("Fake CPU"),
                    Some(16 * 1024 * 1024 * 1024),
                    Some(8),
                )),
            }
        }
    }

    impl ProcessInspector for FakeProcessInspector {
        fn list_processes(&self) -> PlatformResult<Vec<ProcessInfo>> {
            self.processes.clone()
        }

        fn file_version(&self, _path: &Path) -> PlatformResult<Option<VersionMetadata>> {
            self.version.clone()
        }

        fn system_metadata(&self) -> PlatformResult<SystemMetadata> {
            self.system.clone()
        }
    }

    #[test]
    fn fake_process_inspector_supports_manager_detection_inputs() {
        let inspector = FakeProcessInspector {
            processes: Ok(vec![
                ProcessInfo::new(
                    7,
                    Some(3),
                    "ModOrganizer.exe",
                    Some(r"C:\MO2\ModOrganizer.exe"),
                ),
                ProcessInfo::new(9, Some(7), "cmt-rs.exe", Some(r"C:\Tools\cmt-rs.exe")),
            ]),
            version: Ok(Some(VersionMetadata::new(
                SemanticVersion::new(2, 5, 2),
                Some("2.5.2.0"),
            ))),
            ..Default::default()
        };

        let processes = inspector
            .list_processes()
            .expect("fake process list should be available");
        let manager = processes
            .iter()
            .find(|process| process.name == "ModOrganizer.exe")
            .expect("fake MO2 process should be present");
        let version = inspector
            .file_version(manager.executable_path.as_deref().expect("fake exe path"))
            .expect("fake version should be available")
            .expect("fake version metadata should exist");

        assert_eq!(manager.parent_pid, Some(3));
        assert_eq!(version.semantic, SemanticVersion::new(2, 5, 2));
        assert_eq!(version.raw.as_deref(), Some("2.5.2.0"));
    }

    #[test]
    fn fake_process_inspector_returns_typed_system_failures() {
        let inspector = FakeProcessInspector {
            system: Err(PlatformError::new(
                PlatformOperation::ReadSystemMetadata,
                "fake specs",
                PlatformErrorKind::CommandFailed,
                "System metadata read failed.",
            )),
            ..Default::default()
        };

        let error = inspector
            .system_metadata()
            .expect_err("fake system metadata failure should be typed");

        assert_eq!(error.operation, PlatformOperation::ReadSystemMetadata);
        assert_eq!(error.kind, PlatformErrorKind::CommandFailed);
        assert_eq!(error.user_message(), "System metadata read failed.");
    }

    #[cfg(not(windows))]
    #[test]
    fn real_process_operations_are_explicitly_unsupported_off_windows() {
        let inspector = RealProcessInspector::new();

        let process_error = inspector
            .list_processes()
            .expect_err("non-Windows process inspection should be unsupported");
        let version_error = inspector
            .file_version(Path::new("ModOrganizer.exe"))
            .expect_err("non-Windows version metadata should be unsupported");
        let system_error = inspector
            .system_metadata()
            .expect_err("non-Windows system metadata should be unsupported");

        assert_eq!(process_error.kind, PlatformErrorKind::UnsupportedPlatform);
        assert_eq!(version_error.kind, PlatformErrorKind::UnsupportedPlatform);
        assert_eq!(system_error.kind, PlatformErrorKind::UnsupportedPlatform);
        assert_eq!(
            process_error.user_message(),
            "Process inspection is not supported on this platform."
        );
    }
}
