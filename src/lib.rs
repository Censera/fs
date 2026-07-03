use rayon::prelude::*;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};
use walkdir::WalkDir;

#[derive(Debug, thiserror::Error)]
pub enum FsizeError {
    #[error("I/O error for `{path}`: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("Invalid unit specification: `{0}`")]
    InvalidUnit(String),
    #[error("Path does not exist: `{0}`")]
    NotFound(PathBuf),
}

pub struct Color;

impl Color {
    pub fn red() -> &'static str {
        if Self::enabled() {
            "\x1b[1;31m"
        } else {
            ""
        }
    }

    pub fn yellow() -> &'static str {
        if Self::enabled() {
            "\x1b[33m"
        } else {
            ""
        }
    }

    pub fn reset() -> &'static str {
        if Self::enabled() {
            "\x1b[0m"
        } else {
            ""
        }
    }

    fn enabled() -> bool {
        std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
    }
}

#[cfg(unix)]
fn is_virtual_fs(p: &Path) -> bool {
    p.starts_with("/proc") || p.starts_with("/sys") || p.starts_with("/dev")
}

#[cfg(not(unix))]
fn is_virtual_fs(_p: &Path) -> bool {
    false
}

#[derive(Debug, Default)]
pub struct WalkOutcome {
    pub total: u128,
    pub warnings: Vec<String>,
}

pub fn compute_total_size(path: &Path) -> Result<WalkOutcome, FsizeError> {
    let meta = fs::symlink_metadata(path).map_err(|e| FsizeError::Io {
        path: path.to_owned(),
        source: e,
    })?;

    if !meta.file_type().is_dir() {
        return Ok(WalkOutcome {
            total: meta.len() as u128,
            warnings: Vec::new(),
        });
    }

    let warnings: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let push_warning = |msg: String| {
        warnings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(msg);
    };

    let total = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    push_warning(format!("{e}"));
                    return None;
                }
            };

            let p = entry.path();
            if is_virtual_fs(p) {
                return None;
            }

            match entry.metadata() {
                Ok(m) => {
                    if m.is_file() {
                        Some(m.len() as u128)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    push_warning(format!("Cannot access `{}`: {e}", p.display()));
                    return None;
                }
            }
        })
        .sum::<u128>();

    Ok(WalkOutcome {
        total,
        warnings: warnings.into_inner().unwrap_or_else(|p| p.into_inner()),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    fn name(self) -> &'static str {
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
    let truncated = (num * 100.0).trunc() / 100.0;
    let formatted = format!("{:.2}", truncated);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

use chrono::{DateTime, Utc};

pub fn format_mtime(time: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.format("%b %e %H:%M UTC").to_string()
}
