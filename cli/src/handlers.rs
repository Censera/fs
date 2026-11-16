use crate::{
    cli::Args,
    json::{Json, Value},
    measure,
};

use fscore::{Color, FsError, WalkOptions, format_size};
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

fn report(error: &FsError) {
    eprintln!(
        "{}{}error{}: {}",
        Color::bold(),
        Color::red(),
        Color::reset(),
        error
    );
}

/// Ok(false): the run finished, but at least one path failed
fn handle(args: Args) -> Result<bool, FsError> {
    if args.disk_usage {
        disk(&args)?;

        return Ok(true);
    }

    let opts = measure::opts(&args.excludes, args.max_depth, args.follow_symlinks)?;

    if args.paths.is_empty() {
        let paths = measure::entries(Path::new("."), &opts)?;

        list(&paths, &args, &opts, true)
    } else if args.paths.len() == 1 {
        single(&args.paths[0], &args, &opts)?;

        Ok(true)
    } else {
        list(&args.paths, &args, &opts, true)
    }
}

fn single(path: &Path, args: &Args, opts: &WalkOptions) -> Result<(), FsError> {
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

struct Entry {
    path: PathBuf,
    name: String,
    size: u128,
    is_dir: bool,
    hidden: bool,
}

fn list(paths: &[PathBuf], args: &Args, opts: &WalkOptions, sort: bool) -> Result<bool, FsError> {
    let show_total = paths.len() > 1;
    let mut entries = Vec::with_capacity(paths.len());
    let mut json = args.json.then(|| Json::new(args.json_fmt).list());
    let mut total = 0u128;
    let mut clean = true;

    for path in paths {
        let (name, is_dir) = name(path);
        let hidden = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'));

        let size = match size(path, args, opts) {
            Ok(size) => size,
            Err(error) => {
                report(&error);
                clean = false;
                continue;
            }
        };

        total += size;
        entries.push(Entry {
            path: path.to_owned(),
            name,
            size,
            is_dir,
            hidden,
        });
    }

    if let Some(mut list) = json {
        for entry in &entries {
            entry_json(&entry.path, entry.size, args, |fields| list.entry(fields));
        }

        let bytes = total.to_string();
        let formatted = format_size(total, args.in_unit, args.binary);

        list.finish(&[
            ("total_bytes", Value::Str(&bytes)),
            ("total_formatted", Value::Str(&formatted)),
        ]);

        return Ok(clean);
    }

    if sort {
        entries.sort_by(|a, b| {
            let group = |entry: &Entry| {
                if entry.hidden {
                    0
                } else if entry.is_dir {
                    1
                } else {
                    2
                }
            };

            group(a).cmp(&group(b)).then_with(|| {
                if a.hidden {
                    a.name.cmp(&b.name)
                } else {
                    b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name))
                }
            })
        });
    }

    let width = entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(0);

    for entry in &entries {
        let name = format!("{:<width$}", entry.name);
        let size = format(entry.size, args);

        let (name_color, size_color, bold) = if entry.hidden {
            (Color::bright_black(), Color::bright_black(), "")
        } else if entry.is_dir {
            (Color::blue(), Color::green(), Color::bold())
        } else {
            (Color::yellow(), Color::green(), Color::bold())
        };

        println!(
            "{}{}{}    {}{}{}{}",
            name_color,
            name,
            Color::reset(),
            bold,
            size_color,
            size,
            Color::reset(),
        );
    }

    if show_total {
        println!(
            "{:width$}    {}{}{}{}",
            "",
            Color::bold(),
            Color::green(),
            format(total, args),
            Color::reset(),
        );
    }

    Ok(clean)
}

fn size(path: &Path, args: &Args, opts: &WalkOptions) -> Result<u128, FsError> {
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

fn name(path: &Path) -> (String, bool) {
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

    (trim(&name), is_dir)
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

fn disk(args: &Args) -> Result<(), FsError> {
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

    const WIDTH: usize = 80;

    let used = info.used();
    let percent = info.percent_used();
    let used_percent = (percent as usize).min(100);
    let available_percent = 100 - used_percent;

    println!(
        "total {} / free {}",
        format(info.total, args),
        format(info.free, args),
    );

    let used_label = format!("{} | {}%", format(used, args), used_percent);
    let available_label = format!("{} | {}%", format(info.available, args), available_percent,);

    let used_min = used_label.len() + 2;
    let available_min = available_label.len() + 2;
    let width = WIDTH.max(used_min + available_min);

    let used_width = (width * used_percent / 100)
        .max(used_min)
        .min(width - available_min);
    let available_width = width - used_width;

    let color = if percent >= 90.0 {
        Color::red()
    } else if percent >= 70.0 {
        Color::yellow()
    } else {
        ""
    };

    println!(
        "{}{}",
        centered("used", used_width),
        centered("available", available_width),
    );
    println!(
        "{}{}",
        segment(&used_label, used_width, color),
        segment(&available_label, available_width, Color::green()),
    );

    Ok(())
}

fn centered(label: &str, width: usize) -> String {
    let padding = width.saturating_sub(label.len());

    format!(
        "{}{}{}",
        " ".repeat(padding / 2),
        label,
        " ".repeat(padding - padding / 2),
    )
}

fn segment(label: &str, width: usize, color: &str) -> String {
    let padding = width.saturating_sub(label.len() + 2);

    format!(
        "[{}{}{}{}{}]",
        " ".repeat(padding / 2),
        color,
        label,
        Color::reset(),
        " ".repeat(padding - padding / 2),
    )
}
