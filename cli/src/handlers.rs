use crate::{
    cli::Args,
    json::{Json, Value},
    measure,
};

use fscore::{Color, fsError, WalkOptions, format_size};

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub fn run(args: Args) {
    match handle(args) {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            report(&error);

            process::exit(1);
        }
    }
}

fn report(error: &fsError) {
    eprintln!(
        "{}{}error{}: {}",
        Color::bold(),
        Color::red(),
        Color::reset(),
        error
    );
}

/// Ok(false): the run finished, but at least one path failed (already reported).
fn handle(args: Args) -> Result<bool, fsError> {
    if args.disk_usage {
        disk(&args)?;

        return Ok(true);
    }

    let opts = measure::opts(&args.excludes, args.max_depth, args.follow_symlinks)?;

    if args.paths.is_empty() {
        let paths = measure::entries(Path::new("."), &opts)?;

        list(&paths, &args, &opts)
    } else if args.paths.len() == 1 {
        single(&args.paths[0], &args, &opts)?;

        Ok(true)
    } else {
        list(&args.paths, &args, &opts)
    }
}

fn single(path: &Path, args: &Args, opts: &WalkOptions) -> Result<(), fsError> {
    let size = size(path, args, opts)?;

    if args.json {
        entry_json(path, size, args, |fields| {
            Json::new(args.json_fmt).print(fields)
        });

        return Ok(());
    }

    println!("{}", format(size, args));

    Ok(())
}

/// Builds the fields of one entity and hands them to `write`.
fn entry_json(path: &Path, size: u128, args: &Args, write: impl FnOnce(&[(&str, Value)])) {
    let shown = path.display().to_string();
    let bytes = size.to_string();
    let formatted = format_size(size, args.in_unit, args.binary);

    write(&[
        ("path", Value::Str(&shown)),
        ("bytes", Value::Str(&bytes)),
        ("formatted", Value::Str(&formatted)),
    ]);
}

fn list(paths: &[PathBuf], args: &Args, opts: &WalkOptions) -> Result<bool, fsError> {
    let names = paths.iter().map(|path| name(path)).collect::<Vec<_>>();

    let show_total = paths.len() > 1;

    let width = names
        .iter()
        .map(|name| name.chars().count())
        .chain(show_total.then_some("total".len()))
        .max()
        .unwrap_or(0);

    let mut json = args.json.then(|| Json::new(args.json_fmt).list());
    let mut total = 0u128;
    let mut clean = true;

    for (path, name) in paths.iter().zip(names.iter()) {
        let size = match size(path, args, opts) {
            Ok(size) => size,
            Err(error) => {
                report(&error);
                clean = false;

                continue;
            }
        };

        total += size;

        if let Some(list) = json.as_mut() {
            entry_json(path, size, args, |fields| list.entry(fields));

            continue;
        }

        let name = format!("{name:<width$}");

        println!(
            "{}{}{}{}    {}",
            Color::bold(),
            Color::blue(),
            name,
            Color::reset(),
            format(size, args),
        );
    }

    if let Some(list) = json {
        let bytes = total.to_string();
        let formatted = format_size(total, args.in_unit, args.binary);

        list.finish(&[
            ("total_bytes", Value::Str(&bytes)),
            ("total_formatted", Value::Str(&formatted)),
        ]);
    } else if show_total {
        println!(
            "{}{:<width$}{}    {}",
            Color::blue(),
            "total",
            Color::reset(),
            format(total, args),
        );
    }

    Ok(clean)
}

fn size(path: &Path, args: &Args, opts: &WalkOptions) -> Result<u128, fsError> {
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

fn name(path: &Path) -> String {
    let mut name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let is_dir = fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_dir())
        .unwrap_or(false);

    if is_dir {
        name.push('/');
    }

    trim(&name)
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

fn disk(args: &Args) -> Result<(), fsError> {
    #[cfg(unix)]
    let path = PathBuf::from("/");

    #[cfg(windows)]
    let path = {
        let drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());

        PathBuf::from(drive).join("\\")
    };

    let info = measure::disk(&path)?;

    if args.json {
        let total = info.total.to_string();
        let used = info.used().to_string();
        let free = info.free.to_string();
        let available = info.available.to_string();
        let percent = serde_json::to_string(&info.percent_used()).unwrap_or_else(|_| "null".into());

        Json::new(args.json_fmt).print(&[
            ("total", Value::Str(&total)),
            ("used", Value::Str(&used)),
            ("free", Value::Str(&free)),
            ("available", Value::Str(&available)),
            ("percent_used", Value::Num(percent)),
        ]);

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

    label_color("used      ", format(used, args), color);
    label("available ", format(info.available, args));
    label("free      ", format(info.free, args));
    label("total     ", format(info.total, args));
    label_color("used      ", format!("{percent:.1}%",), color);

    Ok(())
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

fn label(name: &str, value: String) {
    println!(
        "{}{}{}{}    {}",
        Color::bold(),
        Color::green(),
        name,
        Color::reset(),
        value,
    );
}
