use clap::Parser;
use fsize_core::{
    Color, FsizeError, Unit, WalkOptions, WalkOutcome, compute_total_size, disk_usage,
    format_mtime, format_size,
};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "fsize (file/folder size)",
    version,
    about = "Display file/directory sizes",
    arg_required_else_help = true
)]
struct Args {
    #[arg(required = true, value_name = "PATH")]
    paths: Vec<PathBuf>,
    #[arg(short = 'b', long = "binary", conflicts_with = "raw")]
    binary: bool,
    #[arg(short = 'r', long = "raw", visible_alias = "byte")]
    raw: bool,

    #[arg(short = 'i', long = "info")]
    info: bool,

    #[arg(short = 'm', long = "metadata", conflicts_with = "disk_usage")]
    metadata: bool,

    #[arg(short = 'd', long = "disk-usage")]
    disk_usage: bool,

    #[arg(
        short = 'u',
        long = "unit",
        value_name = "UNIT",
        conflicts_with = "raw"
    )]
    in_unit: Option<Unit>,

    #[arg(long = "exclude", value_name = "PATTERN")]
    excludes: Vec<String>,

    #[arg(long = "max-depth", value_name = "N")]
    max_depth: Option<usize>,

    #[arg(short = 'L', long = "follow-symlinks")]
    follow_symlinks: bool,

    #[arg(long = "json")]
    json: bool,
}

fn main() {
    let args = Args::parse();

    let excludes = match WalkOptions::compile_excludes(&args.excludes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}error{} {}", Color::red(), Color::reset(), e);
            process::exit(2);
        }
    };

    let walk_opts = WalkOptions {
        max_depth: args.max_depth,
        excludes,
        follow_links: args.follow_symlinks,
    };

    let mut exit_code = 0;
    let mut grand_total: u128 = 0;
    let mut json_entries: Vec<serde_json::Value> = Vec::new();

    for path in &args.paths {
        if args.disk_usage {
            match disk_usage(path) {
                Ok(info) => {
                    if args.json {
                        json_entries.push(serde_json::json!({
                            "path": path.display().to_string(),
                            "total": info.total.to_string(),
                            "used": info.used().to_string(),
                            "free": info.free.to_string(),
                            "available": info.available.to_string(),
                            "percent_used": info.percent_used(),
                        }));
                    } else {
                        let fmt = |b: u128| {
                            if args.raw {
                                b.to_string()
                            } else {
                                format_size(b, args.in_unit, args.binary)
                            }
                        };
                        println!(
                            "{}\ttotal {}\tused {}\tavailable {}\t({:.1}% used)",
                            path.display(),
                            fmt(info.total),
                            fmt(info.used()),
                            fmt(info.available),
                            info.percent_used(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{}error{} {}", Color::red(), Color::reset(), e);
                    exit_code = 1;
                }
            }
            continue;
        }

        let outcome: Result<WalkOutcome, FsizeError> = if args.metadata {
            fs::symlink_metadata(path)
                .map(|m| WalkOutcome {
                    total: m.len() as u128,
                    warnings: Vec::new(),
                    files_scanned: 1,
                })
                .map_err(|e| FsizeError::Io {
                    path: path.to_owned(),
                    source: e,
                })
        } else {
            run_with_progress(path, &walk_opts)
        };

        match outcome {
            Ok(WalkOutcome {
                total, warnings, ..
            }) => {
                for w in &warnings {
                    eprintln!("{}warning{} {}", Color::yellow(), Color::reset(), w);
                }
                if !warnings.is_empty() {
                    exit_code = 1;
                }

                grand_total += total;

                let raw = args.raw;
                let size_str = if raw {
                    total.to_string()
                } else {
                    format_size(total, args.in_unit, args.binary)
                };

                let mut extra_type = None;
                let mut extra_mtime = None;
                if args.info {
                    match fs::symlink_metadata(path) {
                        Ok(meta) => {
                            let ft = meta.file_type();
                            extra_type = Some(if ft.is_dir() {
                                'D'
                            } else if ft.is_symlink() {
                                'L'
                            } else {
                                'F'
                            });
                            if let Ok(mtime) = meta.modified() {
                                extra_mtime = Some(format_mtime(mtime));
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "{}warning{} Cannot read metadata for `{}`: {}",
                                Color::yellow(),
                                Color::reset(),
                                path.display(),
                                e
                            );
                            exit_code = 1;
                        }
                    }
                }

                if args.json {
                    json_entries.push(serde_json::json!({
                        "path": path.display().to_string(),
                        "bytes": total.to_string(),
                        "formatted": size_str,
                        "type": extra_type.map(|c| c.to_string()),
                        "modified": extra_mtime,
                        "warnings": warnings,
                    }));
                } else {
                    let mut output = size_str;
                    if let Some(t) = extra_type {
                        output.push(' ');
                        output.push(t);
                        if let Some(m) = &extra_mtime {
                            output.push(' ');
                            output.push_str(m);
                        }
                    }

                    if args.paths.len() > 1 {
                        println!("{}\t{}", output, path.display());
                    } else {
                        println!("{}", output);
                    }
                }
            }
            Err(e) => {
                eprintln!("{}error{} {}", Color::red(), Color::reset(), e);
                exit_code = 1;
            }
        }
    }

    if args.json {
        let payload = if args.paths.len() > 1 && !args.disk_usage {
            serde_json::json!({
                "entries": json_entries,
                "total_bytes": grand_total.to_string(),
                "total_formatted": if args.raw {
                                  grand_total.to_string()
                              } else {
                                  format_size(grand_total, args.in_unit, args.binary)
                              },
            })
        } else {
            serde_json::json!({ "entries": json_entries })
        };
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else if args.paths.len() > 1 && !args.disk_usage {
        let total_str = if args.raw {
            grand_total.to_string()
        } else {
            format_size(grand_total, args.in_unit, args.binary)
        };
        println!("{}\ttotal", total_str);
    }

    process::exit(exit_code);
}

fn run_with_progress(path: &Path, opts: &WalkOptions) -> Result<WalkOutcome, FsizeError> {
    use std::io::IsTerminal;

    const SPINNER: [&str; 2] = ["·•", "•·"];
    const STUCK_AT_ZERO_HINT_AFTER: Duration = Duration::from_secs(2);

    let counter = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let printed = Arc::new(AtomicBool::new(false));

    let show_progress = std::io::stderr().is_terminal();

    let progress_thread = if show_progress {
        let counter = Arc::clone(&counter);
        let done = Arc::clone(&done);
        let printed = Arc::clone(&printed);
        Some(std::thread::spawn(move || {
            let start = Instant::now();
            let mut frame = 0usize;
            loop {
                std::thread::sleep(Duration::from_millis(150));
                if done.load(Ordering::Relaxed) {
                    break;
                }
                if start.elapsed() > Duration::from_millis(300) {
                    let n = counter.load(Ordering::Relaxed);
                    let elapsed = start.elapsed().as_secs();
                    let spin = SPINNER[frame % SPINNER.len()];
                    frame += 1;

                    let hint = if n == 0 && start.elapsed() > STUCK_AT_ZERO_HINT_AFTER {
                        "(reading directories... Can take a while before the first file turns up)"
                    } else {
                        ""
                    };

                    eprint!(
                        "\r\x1b[2K{}{} scanning{} {} files, {}s {}",
                        Color::yellow(),
                        spin,
                        Color::reset(),
                        n,
                        elapsed,
                        hint,
                    );
                    printed.store(true, Ordering::Relaxed);
                }
            }
        }))
    } else {
        None
    };

    let result = compute_total_size(path, opts, Some(&counter));

    done.store(true, Ordering::Relaxed);
    if let Some(t) = progress_thread {
        let _ = t.join();
        if printed.load(Ordering::Relaxed) {
            eprint!("\r\x1b[2K");
        }
    }

    result
}
