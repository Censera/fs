use jwalk::WalkDir as JWalkDir;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::{fs, io};

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
    #[error("Path does not exist: `{0}`")]
    NotFound(PathBuf),
    #[error("Path is not valid UTF-8: `{0}`")]
    InvalidPath(PathBuf),
    #[error("Disk usage is not supported on this platform: {0}")]
    Unsupported(String),
}

pub struct Color;

impl Color {
    pub fn red() -> &'static str {
        if Self::enabled() { "\x1b[1;31m" } else { "" }
    }

    pub fn yellow() -> &'static str {
        if Self::enabled() { "\x1b[33m" } else { "" }
    }

    pub fn reset() -> &'static str {
        if Self::enabled() { "\x1b[0m" } else { "" }
    }

    fn enabled() -> bool {
        enable_windows_ansi();
        std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
    }
}

#[cfg(windows)]
fn enable_windows_ansi() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        use windows_sys::Win32::System::Console::{
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE,
            SetConsoleMode,
        };
        let handle = GetStdHandle(STD_ERROR_HANDLE);
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    });
}

#[cfg(not(windows))]
fn enable_windows_ansi() {}

#[cfg(unix)]
fn is_virtual_fs(p: &Path) -> bool {
    p.starts_with("/proc") || p.starts_with("/sys") || p.starts_with("/dev")
}

#[cfg(not(unix))]
fn is_virtual_fs(_p: &Path) -> bool {
    false
}

#[cfg(unix)]
fn file_identity(meta: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

#[cfg(windows)]
fn file_identity(meta: &fs::Metadata) -> (u64, u64) {
    use std::os::windows::fs::MetadataExt;
    (
        meta.volume_serial_number().unwrap_or(0) as u64,
        meta.file_index().unwrap_or(0),
    )
}

#[cfg(not(any(unix, windows)))]
fn file_identity(_meta: &fs::Metadata) -> (u64, u64) {
    (0, 0)
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
            .map(|p| {
                glob::Pattern::new(p).map_err(|source| FsizeError::InvalidPattern {
                    pattern: p.clone(),
                    source,
                })
            })
            .collect()
    }
}

#[derive(Debug, Default, Serialize)]
pub struct WalkOutcome {
    pub total: u128,
    pub warnings: Vec<String>,
    pub files_scanned: u64,
}

pub fn compute_total_size(
    path: &Path,
    opts: &WalkOptions,
    progress: Option<&AtomicU64>,
) -> Result<WalkOutcome, FsizeError> {
    let meta = fs::symlink_metadata(path).map_err(|e| FsizeError::Io {
        path: path.to_owned(),
        source: e,
    })?;

    if !meta.file_type().is_dir() {
        if let Some(c) = progress {
            c.fetch_add(1, Ordering::Relaxed);
        }
        return Ok(WalkOutcome {
            total: meta.len() as u128,
            warnings: Vec::new(),
            files_scanned: 1,
        });
    }

    let warnings: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    fn push(target: &Mutex<Vec<String>>, msg: String) {
        target
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(msg);
    }

    let excludes = opts.excludes.clone();
    let follow_links = opts.follow_links;
    let visited: Arc<Mutex<HashSet<(u64, u64)>>> = Arc::new(Mutex::new(HashSet::new()));
    let warnings_pr = Arc::clone(&warnings);
    let visited_pr = Arc::clone(&visited);
    let mut walker = JWalkDir::new(path)
        .follow_links(follow_links)
        .skip_hidden(false)
        .process_read_dir(move |_depth, _parent, _state, children| {
            children.retain(|entry_result| {
                let Ok(entry) = entry_result else {
                    return true;
                };
                let p = entry.path();
                if is_virtual_fs(&p) {
                    return false;
                }
                let file_name = entry.file_name().to_string_lossy();
                if excludes
                    .iter()
                    .any(|pat| pat.matches(&file_name) || pat.matches_path(&p))
                {
                    return false;
                }

                if follow_links
                    && let Ok(meta) = fs::metadata(&p)
                    && meta.is_dir()
                {
                    let id = file_identity(&meta);
                    let mut seen = visited_pr.lock().unwrap_or_else(|p| p.into_inner());
                    if !seen.insert(id) {
                        drop(seen);
                        push(
                            &warnings_pr,
                            format!(
                                "Skipped `{}`: already-visited directory (symlink cycle)",
                                p.display()
                            ),
                        );
                        return false;
                    }
                }

                true
            });
        });
    if let Some(depth) = opts.max_depth {
        walker = walker.max_depth(depth);
    }

    let total = walker
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    push(&warnings, format!("{e}"));
                    return None;
                }
            };

            match entry.metadata() {
                Ok(m) => {
                    if m.is_file() {
                        if let Some(c) = progress {
                            c.fetch_add(1, Ordering::Relaxed);
                        }
                        Some(m.len() as u128)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    push(
                        &warnings,
                        format!("Cannot access `{}`: {e}", entry.path().display()),
                    );
                    None
                }
            }
        })
        .sum::<u128>();

    let warnings = Arc::try_unwrap(warnings)
        .map(|m| m.into_inner().unwrap_or_else(|p| p.into_inner()))
        .unwrap_or_else(|arc| arc.lock().unwrap_or_else(|p| p.into_inner()).clone());
    let files_scanned = progress.map(|c| c.load(Ordering::Relaxed)).unwrap_or(0);

    Ok(WalkOutcome {
        total,
        warnings,
        files_scanned,
    })
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
            0.0
        } else {
            (self.used() as f64 / self.total as f64) * 100.0
        }
    }
}

#[cfg(unix)]
pub fn disk_usage(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path_str = path
        .to_str()
        .ok_or_else(|| FsizeError::InvalidPath(path.to_owned()))?;
    let c_path = CString::new(path_str).map_err(|_| FsizeError::InvalidPath(path.to_owned()))?;

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    let rc = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return Err(FsizeError::Io {
            path: path.to_owned(),
            source: io::Error::last_os_error(),
        });
    }

    let stat = unsafe { stat.assume_init() };

    let frsize = stat.f_frsize as u128;
    let total = stat.f_blocks as u128 * frsize;
    let free = stat.f_bfree as u128 * frsize;
    let available = stat.f_bavail as u128 * frsize;

    Ok(DiskUsageInfo {
        total,
        free,
        available,
    })
}

#[cfg(windows)]
pub fn disk_usage(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_available: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free: u64 = 0;

    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_available,
            &mut total_bytes,
            &mut total_free,
        )
    };

    if ok == 0 {
        return Err(FsizeError::Io {
            path: path.to_owned(),
            source: io::Error::last_os_error(),
        });
    }

    Ok(DiskUsageInfo {
        total: total_bytes as u128,
        free: total_free as u128,
        available: free_available as u128,
    })
}

#[cfg(not(any(unix, windows)))]
pub fn disk_usage(_path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    Err(FsizeError::Unsupported(
        "disk usage queries need statvfs (unix) or GetDiskFreeSpaceExW (windows)".to_string(),
    ))
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "b" => Ok(Unit::B),
            "kb" => Ok(Unit::KB),
            "mb" => Ok(Unit::MB),
            "gb" => Ok(Unit::GB),
            "tb" => Ok(Unit::TB),
            "pb" => Ok(Unit::PB),
            "eb" => Ok(Unit::EB),
            "zb" => Ok(Unit::ZB),
            "yb" => Ok(Unit::YB),
            "rb" => Ok(Unit::RB),
            "qb" => Ok(Unit::QB),
            "kib" => Ok(Unit::KiB),
            "mib" => Ok(Unit::MiB),
            "gib" => Ok(Unit::GiB),
            "tib" => Ok(Unit::TiB),
            "pib" => Ok(Unit::PiB),
            "eib" => Ok(Unit::EiB),
            "zib" => Ok(Unit::ZiB),
            "yib" => Ok(Unit::YiB),
            "rib" => Ok(Unit::RiB),
            "qib" => Ok(Unit::QiB),
            other => Err(FsizeError::InvalidUnit(other.to_string())),
        }
    }
}

impl Unit {
    const fn divisor(self) -> u128 {
        match self {
            Unit::B => 1,
            Unit::KB => 1000,
            Unit::MB => 1_000_000,
            Unit::GB => 1_000_000_000,
            Unit::TB => 1_000_000_000_000,
            Unit::PB => 1_000_000_000_000_000,
            Unit::EB => 1_000_000_000_000_000_000,
            Unit::ZB => 1_000_000_000_000_000_000_000,
            Unit::YB => 1_000_000_000_000_000_000_000_000,
            Unit::RB => 1_000_000_000_000_000_000_000_000_000,
            Unit::QB => 1_000_000_000_000_000_000_000_000_000_000,
            Unit::KiB => 1024,
            Unit::MiB => 1024 * 1024,
            Unit::GiB => 1024 * 1024 * 1024,
            Unit::TiB => 1024 * 1024 * 1024 * 1024,
            Unit::PiB => 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::EiB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::ZiB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::YiB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::RiB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            Unit::QiB => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Unit::B => "B",
            Unit::KB => "KB",
            Unit::MB => "MB",
            Unit::GB => "GB",
            Unit::TB => "TB",
            Unit::PB => "PB",
            Unit::EB => "EB",
            Unit::ZB => "ZB",
            Unit::YB => "YB",
            Unit::RB => "RB",
            Unit::QB => "QB",
            Unit::KiB => "KiB",
            Unit::MiB => "MiB",
            Unit::GiB => "GiB",
            Unit::TiB => "TiB",
            Unit::PiB => "PiB",
            Unit::EiB => "EiB",
            Unit::ZiB => "ZiB",
            Unit::YiB => "YiB",
            Unit::RiB => "RiB",
            Unit::QiB => "QiB",
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
        format!("{} {}", bytes, unit.name())
    } else {
        let value = bytes as f64 / divisor as f64;
        format!("{} {}", format_pre(value), unit.name())
    }
}

fn format_pre(num: f64) -> String {
    format!("{:.2}", num)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

use chrono::{DateTime, Utc};

pub fn format_mtime(time: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.format("%b %e %H:%M UTC").to_string()
}
