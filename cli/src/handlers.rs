use crate::{cli::Args, measure};

use fsize_core::{Color, FsizeError, WalkOptions, format_size};

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub fn run(args: Args) {
    if let Err(error) = handle(args) {
        eprintln!(
            "{}{}error{}: {}",
            Color::bold(),
            Color::red(),
            Color::reset(),
            error
        );

        process::exit(1);
    }
}

fn handle(args: Args) -> Result<(), FsizeError> {
    if args.disk_usage {
        return disk(&args);
    }

    let opts = measure::opts(&args.excludes, args.max_depth, args.follow_symlinks)?;

    if args.paths.is_empty() {
        let paths = measure::entries(Path::new("."), &opts)?;
        list(&paths, &args, &opts)?;
    } else if args.paths.len() == 1 {
        single(&args.paths[0], &args, &opts)?;
    } else {
        list(&args.paths, &args, &opts)?;
    }

    Ok(())
}

fn single(path: &Path, args: &Args, opts: &WalkOptions) -> Result<(), FsizeError> {
    let size = size(path, args, opts)?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "path": path,
                "bytes": size,
                "formatted": format_size(
                    size,
                    args.in_unit,
                    args.binary,
                ),
            })
        );

        return Ok(());
    }

    println!("{}", format(size, args));

    Ok(())
}

fn list(paths: &[PathBuf], args: &Args, opts: &WalkOptions) -> Result<(), FsizeError> {
    let names = paths
        .iter()
        .map(|path| name(path))
        .collect::<Result<Vec<_>, _>>()?;

    let width = names
        .iter()
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(0);

    let mut entries = Vec::with_capacity(paths.len());

    for (path, name) in paths.iter().zip(names.iter()) {
        let size = size(path, args, opts)?;

        if args.json {
            entries.push(serde_json::json!({
                "path": path,
                "bytes": size.to_string(),
                "formatted": format_size(
                    size,
                    args.in_unit,
                    args.binary,
                ),
            }));

            continue;
        }

        let name = format!("{name:<width$}");

        println!(
            "{}{}{}{}{}    {}",
            Color::bold(),
            Color::blue(),
            name,
            Color::reset(),
            "",
            format(size, args),
        );
    }

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "entries": entries,
            })
        );
    }

    Ok(())
}

fn size(path: &Path, args: &Args, opts: &WalkOptions) -> Result<u128, FsizeError> {
    if args.metadata {
        return measure::metadata(path);
    }

    let outcome = measure::size(path, opts)?;

    for warning in &outcome.warnings {
        eprintln!(
            "{}{}warning{}: {}",
            Color::bold(),
            Color::yellow(),
            Color::reset(),
            warning
        );
    }

    Ok(outcome.total)
}

fn format(size: u128, args: &Args) -> String {
    if args.raw {
        size.to_string()
    } else {
        format_size(size, args.in_unit, args.binary)
    }
}

fn name(path: &Path) -> Result<String, FsizeError> {
    let mut name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let meta = fs::symlink_metadata(path).map_err(|source| FsizeError::Io {
        path: path.to_owned(),
        source,
    })?;

    if meta.file_type().is_dir() {
        name.push('/');
    }

    Ok(trim(&name))
}

fn trim(name: &str) -> String {
    const WIDTH: usize = 20;

    if name.chars().count() <= WIDTH {
        return name.to_owned();
    }

    let mut name = name.chars().take(WIDTH - 3).collect::<String>();

    name.push_str("...");
    name
}

fn disk(args: &Args) -> Result<(), FsizeError> {
    #[cfg(unix)]
    let path = Path::new("/");

    #[cfg(windows)]
    let path = {
        let drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());

        PathBuf::from(drive).join("\\")
    };

    let info = measure::disk(path)?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "total": info.total.to_string(),
                "used": info.used().to_string(),
                "free": info.free.to_string(),
                "available": info.available.to_string(),
                "percent_used": info.percent_used(),
            })
        );

        return Ok(());
    }

    let used = info.used();
    let percent = info.percent_used();

    let color = if percent >= 90.0 {
        Color::red()
    } else if percent >= 70.0 {
        Color::yellow()
    } else {
        ""
    };

    label("total", format(info.total, args));
    label_color("used", format(used, args), color);
    label("free", format(info.free, args));
    label("available", format(info.available, args));
    label_color("used", format!("{percent:.1}%",), color);

    Ok(())
}

fn label(name: &str, value: String) {
    println!(
        "{}{}{}    {}",
        Color::bold(),
        Color::green(),
        name,
        Color::reset(),
    );

    print!("{}\r", value);

    println!();
}

fn label_color(name: &str, value: String, color: &str) {
    println!(
        "{}{}{}    {}{}{}",
        Color::bold(),
        Color::green(),
        name,
        color,
        value,
        Color::reset(),
    );
}
