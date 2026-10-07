use fsize_core::{
    Color, DiskUsageInfo, FsizeError, WalkOptions, WalkOutcome, compute_total_size, disk_usage,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub fn opts(
    excludes: &[String],
    max_depth: Option<usize>,
    follow_links: bool,
) -> Result<WalkOptions, FsizeError> {
    Ok(WalkOptions {
        max_depth,
        excludes: WalkOptions::compile_excludes(excludes)?,
        follow_links,
    })
}

pub fn size(path: &Path, opts: &WalkOptions) -> Result<WalkOutcome, FsizeError> {
    progress(path, opts)
}

pub fn metadata(path: &Path) -> Result<u128, FsizeError> {
    fsize_core::metadata_size(path)
}

pub fn disk(path: &Path) -> Result<DiskUsageInfo, FsizeError> {
    disk_usage(path)
}

pub fn entries(path: &Path, opts: &WalkOptions) -> Result<Vec<PathBuf>, FsizeError> {
    let mut out = Vec::new();

    for entry in std::fs::read_dir(path).map_err(|source| FsizeError::Io {
        path: path.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| FsizeError::Io {
            path: path.to_owned(),
            source,
        })?;

        let path = entry.path();

        if opts.excludes.iter().any(|pattern| {
            path.file_name()
                .is_some_and(|name| pattern.matches(&name.to_string_lossy()))
                || pattern.matches_path(&path)
        }) {
            continue;
        }

        out.push(path);
    }

    out.sort();
    Ok(out)
}

fn progress(path: &Path, opts: &WalkOptions) -> Result<WalkOutcome, FsizeError> {
    use std::io::IsTerminal;

    const SPIN: [&str; 7] = ["/  ", "// ", "///", " //", "  /", "   ", "   "];

    let count = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let shown = Arc::new(AtomicBool::new(false));

    let active = std::io::stderr().is_terminal();

    let thread = if active {
        let count = Arc::clone(&count);
        let done = Arc::clone(&done);
        let shown = Arc::clone(&shown);

        Some(std::thread::spawn(move || {
            let start = Instant::now();
            let mut frame = 0;

            loop {
                std::thread::sleep(Duration::from_millis(150));

                if done.load(Ordering::Relaxed) {
                    break;
                }

                if start.elapsed() < Duration::from_millis(300) {
                    continue;
                }

                let spin = SPIN[frame % SPIN.len()];
                frame += 1;

                eprint!(
                    "\r\x1b[2K{}{}{} Scanning {} files, {}s{}",
                    Color::bold(),
                    Color::yellow(),
                    spin,
                    count.load(Ordering::Relaxed),
                    start.elapsed().as_secs(),
                    Color::reset(),
                );

                shown.store(true, Ordering::Relaxed);
            }
        }))
    } else {
        None
    };

    let result = compute_total_size(path, opts, Some(&count));

    done.store(true, Ordering::Relaxed);

    if let Some(thread) = thread {
        let _ = thread.join();

        if shown.load(Ordering::Relaxed) {
            eprint!("\r\x1b[2K");
        }
    }

    result
}
