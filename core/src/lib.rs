use jwalk::WalkDir as JWalkDir;
use rayon::prelude::*;
use serde::Serialize;

use std::collections::HashSet;
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::{env, fs};

#[derive(Debug, thiserror::Error)]
pub enum FsizeError {
    #[error("I/O error for `{path}`: {source}")]
    Io { path: PathBuf, source: io::Error },

    #[error("Invalid unit specification: `{0}`")]
    InvalidUnit(String),

    #[error("Invalid exclude pattern `{pattern}`: {source}")]
    InvalidPattern {
        pattern: String,
        source: glob::PatternError,
    },

    #[error("Path is not valid UTF-8: `{0}`")]
    InvalidPath(PathBuf),

    #[error("Disk usage is not supported on this platform: {0}")]
    Unsupported(String),
}

pub struct Color;

impl Color {
    fn enabled() -> bool {
        enable_windows_ansi();

        if env::var_os("NO_COLOR").is_some() {
            return false;
        }

        if env::var_os("FORCE_COLOR").is_some() {
            return true;
        }

        if env::var("TERM").ok().as_deref() == Some("dumb") {
            return false;
        }

        std::io::stdout().is_terminal() || std::io::stderr().is_terminal()
    }

    fn code(code: &'static str) -> &'static str {
        if Self::enabled() { code } else { "" }
    }

    pub fn reset() -> &'static str {
        Self::code("\x1b[0m")
    }

    pub fn dim() -> &'static str {
        Self::code("\x1b[2m")
    }

    pub fn bold() -> &'static str {
        Self::code("\x1b[1m")
    }

    pub fn italic() -> &'static str {
        Self::code("\x1b[3m")
    }

    pub fn black() -> &'static str {
        Self::code("\x1b[30m")
    }

    pub fn red() -> &'static str {
        Self::code("\x1b[31m")
    }

    pub fn green() -> &'static str {
        Self::code("\x1b[32m")
    }

    pub fn yellow() -> &'static str {
        Self::code("\x1b[33m")
    }

    pub fn blue() -> &'static str {
        Self::code("\x1b[34m")
    }

    pub fn magenta() -> &'static str {
        Self::code("\x1b[35m")
    }

    pub fn cyan() -> &'static str {
        Self::code("\x1b[36m")
    }

    pub fn white() -> &'static str {
        Self::code("\x1b[37m")
    }

    pub fn bright_black() -> &'static str {
        Self::code("\x1b[90m")
    }

    pub fn bright_red() -> &'static str {
        Self::code("\x1b[91m")
    }

    pub fn bright_green() -> &'static str {
        Self::code("\x1b[92m")
    }

    pub fn bright_yellow() -> &'static str {
        Self::code("\x1b[93m")
    }

    pub fn bright_blue() -> &'static str {
        Self::code("\x1b[94m")
    }

    pub fn bright_magenta() -> &'static str {
        Self::code("\x1b[95m")
    }

    pub fn bright_cyan() -> &'static str {
        Self::code("\x1b[96m")
    }

    pub fn bright_white() -> &'static str {
        Self::code("\x1b[97m")
    }
}

#[cfg(windows)]
fn enable_windows_ansi() {
    use std::sync::Once;

    static INIT: Once = Once::new();

    INIT.call_once(|| unsafe {
        use windows_sys::Win32::System::Console::{
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE,
            STD_OUTPUT_HANDLE, SetConsoleMode,
        };

        for handle in [
            GetStdHandle(STD_OUTPUT_HANDLE),
            GetStdHandle(STD_ERROR_HANDLE),
        ] {
            let mut mode = 0u32;

            if GetConsoleMode(handle, &mut mode) != 0 {
                let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    });
}

#[cfg(not(windows))]
fn enable_windows_ansi() {}

#[cfg(unix)]
fn is_virtual_fs(path: &Path) -> bool {
    path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/dev")
}

#[cfg(not(unix))]
fn is_virtual_fs(_path: &Path) -> bool {
    false
}

#[cfg(unix)]
fn hardlink_id(meta: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;

    (meta.nlink() > 1).then(|| (meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn hardlink_id(_meta: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

#[cfg(unix)]
fn dir_identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;

    let meta = fs::metadata(path).ok()?;

    Some((meta.dev(), meta.ino()))
}

#[cfg(windows)]
fn dir_identity(path: &Path) -> Option<(u64, u64)> {
    use std::fs::OpenOptions;
    use std::mem::zeroed;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, GetFileInformationByHandle,
    };

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .ok()?;

    let handle = file.as_raw_handle();

    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };

    let ok = unsafe { GetFileInformationByHandle(handle as _, &mut info) };

    if ok == 0 {
        return None;
    }

    let index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;

    Some((info.dwVolumeSerialNumber as u64, index))
}

#[cfg(not(any(unix, windows)))]
fn dir_identity(_path: &Path) -> Option<(u64, u64)> {
    None
}

fn is_symlink_cycle(path: &Path) -> bool {
    let Some(target) = dir_identity(path) else {
        return false;
    };

    path.ancestors()
        .skip(1)
        .any(|ancestor| dir_identity(ancestor) == Some(target))
}

#[derive(Default)]
pub struct WalkOptions {
    pub max_depth: Option<usize>,
    pub excludes: Vec<glob::Pattern>,
    pub follow_links: bool,
}

impl WalkOptions {
    pub fn compile_excludes(patterns: &[String]) -> Result<Vec<glob::Pattern>, FsizeError> {
        patterns
            .iter()
            .map(|pattern| {
                glob::Pattern::new(pattern).map_err(|source| FsizeError::InvalidPattern {
                    pattern: pattern.clone(),
                    source,
                })
            })
            .collect()
    }

    pub fn excluded(&self, path: &Path) -> bool {
        excluded(path, &self.excludes)
    }
}

fn excluded(path: &Path, patterns: &[glob::Pattern]) -> bool {
    let name = path.file_name().map(|name| name.to_string_lossy());

    patterns.iter().any(|pattern| {
        name.as_deref().is_some_and(|name| pattern.matches(name)) || pattern.matches_path(path)
    })
}

#[derive(Debug, Default, Serialize)]
pub struct WalkOutcome {
    pub total: u128,
    pub warnings: Vec<String>,
}

pub fn compute_total_size(
    path: &Path,
    opts: &WalkOptions,
    progress: Option<&AtomicU64>,
) -> Result<WalkOutcome, FsizeError> {
    let meta = if opts.follow_links {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    }
    .map_err(|source| FsizeError::Io {
        path: path.to_owned(),
        source,
    })?;

    if !meta.is_dir() {
        if let Some(counter) = progress {
            counter.fetch_add(1, Ordering::Relaxed);
        }

        return Ok(WalkOutcome {
            total: meta.len() as u128,
            warnings: Vec::new(),
        });
    }

    let warnings = Arc::new(Mutex::new(Vec::<String>::new()));

    fn push(target: &Mutex<Vec<String>>, message: String) {
        target
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(message);
    }

    let excludes = opts.excludes.clone();
    let follow_links = opts.follow_links;
    let warnings_filter = Arc::clone(&warnings);

    let mut walker = JWalkDir::new(path)
        .follow_links(follow_links)
        .skip_hidden(false)
        .process_read_dir(move |_depth, _parent, _state, children| {
            children.retain(|entry_result| {
                let Ok(entry) = entry_result else {
                    return true;
                };

                let entry_path = entry.path();

                if is_virtual_fs(&entry_path) || excluded(&entry_path, &excludes) {
                    return false;
                }

                if follow_links
                    && entry.path_is_symlink()
                    && entry.file_type().is_dir()
                    && is_symlink_cycle(&entry_path)
                {
                    push(
                        &warnings_filter,
                        format!(
                            "Skipped `{}`: already-visited directory (symlink cycle)",
                            entry_path.display()
                        ),
                    );

                    return false;
                }

                true
            });
        });

    if let Some(depth) = opts.max_depth {
        walker = walker.max_depth(depth);
    }

    let hardlinks = Mutex::new(HashSet::<(u64, u64)>::new());

    let total = walker
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,

                Err(error) => {
                    if let Some(ancestor) = error.loop_ancestor() {
                        let child = error.path().unwrap_or_else(|| Path::new(""));

                        push(
                            &warnings,
                            format!(
                                "Skipped `{}`: already-visited directory \
                                 (symlink cycle via `{}`)",
                                child.display(),
                                ancestor.display()
                            ),
                        );
                    } else if let Some(path) = error.path() {
                        push(
                            &warnings,
                            format!("Cannot access `{}`: {error}", path.display()),
                        );
                    } else {
                        push(&warnings, format!("{error}"));
                    }

                    return None;
                }
            };

            if let Some(error) = entry.read_children_error.as_ref() {
                push(
                    &warnings,
                    format!("Cannot access `{}`: {error}", entry.path().display()),
                );
            }

            let meta = match entry.metadata() {
                Ok(meta) => meta,

                Err(error) => {
                    push(
                        &warnings,
                        format!("Cannot access `{}`: {error}", entry.path().display()),
                    );

                    return None;
                }
            };

            if !meta.is_file() {
                return None;
            }

            if let Some(id) = hardlink_id(&meta) {
                let mut seen = hardlinks
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                if !seen.insert(id) {
                    return None;
                }
            }

            if let Some(counter) = progress {
                counter.fetch_add(1, Ordering::Relaxed);
            }

            Some(meta.len() as u128)
        })
        .sum::<u128>();

    let warnings = Arc::try_unwrap(warnings)
        .map(|mutex| {
            mutex
                .into_inner()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        })
        .unwrap_or_else(|arc| {
            arc.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        });

    Ok(WalkOutcome { total, warnings })
}

pub fn metadata_size(path: &Path) -> Result<u128, FsizeError> {
    let meta = fs::symlink_metadata(path).map_err(|source| FsizeError::Io {
        path: path.to_owned(),
        source,
    })?;

    Ok(meta.len() as u128)
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiskUsageInfo {
    pub total: u128,
    pub free: u128,
    pub available: u128,
}

impl DiskUsageInfo {
    pub fn used(&self) -> u128 {
        self.total.saturating_sub(self.free)
    }

    pub fn percent_used(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }

        self.used() as f64 / self.total as f64 * 100.0
    }
}

#[cfg(unix)]
pub fn disk_usage(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path = path
        .to_str()
        .ok_or_else(|| FsizeError::InvalidPath(path.to_owned()))?;

    let path = CString::new(path).map_err(|_| FsizeError::InvalidPath(PathBuf::from(path)))?;

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };

    if result != 0 {
        return Err(FsizeError::Io {
            path: PathBuf::from(path.to_string_lossy().into_owned()),
            source: io::Error::last_os_error(),
        });
    }

    let stat = unsafe { stat.assume_init() };
    let size = stat.f_frsize as u128;

    Ok(DiskUsageInfo {
        total: stat.f_blocks as u128 * size,
        free: stat.f_bfree as u128 * size,
        available: stat.f_bavail as u128 * size,
    })
}

#[cfg(windows)]
pub fn disk_usage(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let mut available = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;

    let result =
        unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };

    if result == 0 {
        return Err(FsizeError::Io {
            path: path.to_owned(),
            source: io::Error::last_os_error(),
        });
    }

    Ok(DiskUsageInfo {
        total: total as u128,
        free: free as u128,
        available: available as u128,
    })
}

#[cfg(not(any(unix, windows)))]
pub fn disk_usage(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    Err(FsizeError::Unsupported(format!(
        "disk usage queries need statvfs (unix) or GetDiskFreeSpaceExW (windows): {}",
        path.display()
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
    EB,
    ZB,
    YB,
    RB,
    QB,
    KiB,
    MiB,
    GiB,
    TiB,
    PiB,
    EiB,
    ZiB,
    YiB,
    RiB,
    QiB,
}

impl std::str::FromStr for Unit {
    type Err = FsizeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "b" => Ok(Self::B),
            "kb" => Ok(Self::KB),
            "mb" => Ok(Self::MB),
            "gb" => Ok(Self::GB),
            "tb" => Ok(Self::TB),
            "pb" => Ok(Self::PB),
            "eb" => Ok(Self::EB),
            "zb" => Ok(Self::ZB),
            "yb" => Ok(Self::YB),
            "rb" => Ok(Self::RB),
            "qb" => Ok(Self::QB),
            "kib" => Ok(Self::KiB),
            "mib" => Ok(Self::MiB),
            "gib" => Ok(Self::GiB),
            "tib" => Ok(Self::TiB),
            "pib" => Ok(Self::PiB),
            "eib" => Ok(Self::EiB),
            "zib" => Ok(Self::ZiB),
            "yib" => Ok(Self::YiB),
            "rib" => Ok(Self::RiB),
            "qib" => Ok(Self::QiB),
            other => Err(FsizeError::InvalidUnit(other.to_owned())),
        }
    }
}

impl Unit {
    const fn divisor(self) -> u128 {
        match self {
            Self::B => 1,

            Self::KB => 1_000,
            Self::MB => 1_000_000,
            Self::GB => 1_000_000_000,
            Self::TB => 1_000_000_000_000,
            Self::PB => 1_000_000_000_000_000,
            Self::EB => 1_000_000_000_000_000_000,
            Self::ZB => 1_000_000_000_000_000_000_000,
            Self::YB => 1_000_000_000_000_000_000_000_000,
            Self::RB => 1_000_000_000_000_000_000_000_000_000,
            Self::QB => 1_000_000_000_000_000_000_000_000_000_000,

            Self::KiB => 1_024,
            Self::MiB => 1_024 * 1_024,
            Self::GiB => 1_024 * 1_024 * 1_024,
            Self::TiB => 1_024 * 1_024 * 1_024 * 1_024,
            Self::PiB => 1_024 * 1_024 * 1_024 * 1_024 * 1_024,
            Self::EiB => 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024,
            Self::ZiB => 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024,
            Self::YiB => 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024,
            Self::RiB => 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024,
            Self::QiB => {
                1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024 * 1_024
            }
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::B => "B",
            Self::KB => "KB",
            Self::MB => "MB",
            Self::GB => "GB",
            Self::TB => "TB",
            Self::PB => "PB",
            Self::EB => "EB",
            Self::ZB => "ZB",
            Self::YB => "YB",
            Self::RB => "RB",
            Self::QB => "QB",
            Self::KiB => "KiB",
            Self::MiB => "MiB",
            Self::GiB => "GiB",
            Self::TiB => "TiB",
            Self::PiB => "PiB",
            Self::EiB => "EiB",
            Self::ZiB => "ZiB",
            Self::YiB => "YiB",
            Self::RiB => "RiB",
            Self::QiB => "QiB",
        }
    }
}

pub fn format_size(bytes: u128, unit: Option<Unit>, binary: bool) -> String {
    let unit = unit.unwrap_or_else(|| {
        if binary {
            if bytes >= Unit::QiB.divisor() {
                Unit::QiB
            } else if bytes >= Unit::RiB.divisor() {
                Unit::RiB
            } else if bytes >= Unit::YiB.divisor() {
                Unit::YiB
            } else if bytes >= Unit::ZiB.divisor() {
                Unit::ZiB
            } else if bytes >= Unit::EiB.divisor() {
                Unit::EiB
            } else if bytes >= Unit::PiB.divisor() {
                Unit::PiB
            } else if bytes >= Unit::TiB.divisor() {
                Unit::TiB
            } else if bytes >= Unit::GiB.divisor() {
                Unit::GiB
            } else if bytes >= Unit::MiB.divisor() {
                Unit::MiB
            } else if bytes >= Unit::KiB.divisor() {
                Unit::KiB
            } else {
                Unit::B
            }
        } else if bytes >= Unit::QB.divisor() {
            Unit::QB
        } else if bytes >= Unit::RB.divisor() {
            Unit::RB
        } else if bytes >= Unit::YB.divisor() {
            Unit::YB
        } else if bytes >= Unit::ZB.divisor() {
            Unit::ZB
        } else if bytes >= Unit::EB.divisor() {
            Unit::EB
        } else if bytes >= Unit::PB.divisor() {
            Unit::PB
        } else if bytes >= Unit::TB.divisor() {
            Unit::TB
        } else if bytes >= Unit::GB.divisor() {
            Unit::GB
        } else if bytes >= Unit::MB.divisor() {
            Unit::MB
        } else if bytes >= Unit::KB.divisor() {
            Unit::KB
        } else {
            Unit::B
        }
    });

    let divisor = unit.divisor();

    if divisor == 1 {
        return format!("{} {}", bytes, unit.name());
    }

    let value = bytes as f64 / divisor as f64;

    format!("{} {}", format_pre(value), unit.name())
}

fn format_pre(value: f64) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
