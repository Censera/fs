use fsize_core::Unit;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process;

const VERSION_NAME: &str = "MOLE";

#[derive(Debug)]
pub struct Args {
    pub paths: Vec<PathBuf>,
    pub metadata: bool,
    pub disk_usage: bool,
    pub follow_symlinks: bool,
    pub excludes: Vec<String>,
    pub max_depth: Option<usize>,
    pub binary: bool,
    pub raw: bool,
    pub b_flag: bool,
    pub in_unit: Option<Unit>,
    pub json: bool,
    pub json_fmt: bool,
}

pub fn parse() -> Args {
    let mut args = Args {
        paths: Vec::new(),
        metadata: false,
        disk_usage: false,
        follow_symlinks: false,
        excludes: Vec::new(),
        max_depth: None,
        binary: false,
        raw: false,
        b_flag: false,
        in_unit: None,
        json: false,
        json_fmt: false,
    };

    let mut iter = env::args_os().skip(1);
    let mut options = true;

    while let Some(arg) = iter.next() {
        if options && arg == OsStr::new("--") {
            options = false;
            continue;
        }

        if !options {
            path(arg, &mut args);
            continue;
        }

        let Some(text) = arg.to_str() else {
            path(arg, &mut args);
            continue;
        };

        if text == "-" || !text.starts_with('-') {
            path(arg, &mut args);
        } else if text.starts_with("--") {
            long(text, &mut iter, &mut args);
        } else {
            short(text, &mut iter, &mut args);
        }
    }

    validate(&args);

    args.raw |= args.b_flag;

    args
}

fn path(arg: OsString, args: &mut Args) {
    if args.disk_usage {
        error("`--disk-usage` does not accept paths");
    }

    args.paths.push(PathBuf::from(arg));
}

fn long(arg: &str, iter: &mut impl Iterator<Item = OsString>, args: &mut Args) {
    let (name, attached) = match arg.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (arg, None),
    };

    match name {
        "--help" => {
            no_value(name, attached);
            help();
        }

        "--version" => {
            no_value(name, attached);
            version();
        }

        "--metadata" => {
            check_disk(args, name);
            no_value(name, attached);
            args.metadata = true;
        }

        "--disk-usage" => {
            set_disk(args, attached);
        }

        "--follow-symlinks" => {
            check_disk(args, name);
            no_value(name, attached);
            args.follow_symlinks = true;
        }

        "--binary" => {
            check_disk(args, name);
            no_value(name, attached);
            args.binary = true;
        }

        "--raw" | "--byte" => {
            check_disk(args, name);
            no_value(name, attached);
            args.raw = true;
        }

        "--json" => {
            match attached {
                None => {}
                Some("fmt") => args.json_fmt = true,
                Some(value) => error(&format!(
                    "invalid value for `--json`: `{value}` (only `fmt` is accepted)"
                )),
            }

            args.json = true;
        }

        "--exclude" => {
            check_disk(args, name);

            let value = value(name, attached, iter);

            for pattern in value.split('|') {
                if pattern.is_empty() {
                    error("empty exclude pattern");
                }

                args.excludes.push(pattern.to_owned());
            }
        }

        "--max-depth" => {
            check_disk(args, name);

            let value = value(name, attached, iter);

            args.max_depth = Some(number(name, &value));
        }

        "--unit" => {
            check_disk(args, name);

            let value = value(name, attached, iter);

            args.in_unit = Some(unit(name, &value));
        }

        _ => error(&format!("unknown option: `{arg}`")),
    }
}

fn short(arg: &str, iter: &mut impl Iterator<Item = OsString>, args: &mut Args) {
    let chars = arg[1..].chars().collect::<Vec<_>>();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            'h' => help(),
            'V' => version(),

            'm' => {
                check_disk(args, "-m");
                args.metadata = true;
            }

            'd' => {
                set_disk(args, None);
            }

            'L' => {
                check_disk(args, "-L");
                args.follow_symlinks = true;
            }

            'b' => {
                check_disk(args, "-b");
                args.b_flag = true;
            }

            'r' => {
                check_disk(args, "-r");
                args.raw = true;
            }

            'u' => {
                check_disk(args, "-u");

                let mut value = chars[i + 1..].iter().collect::<String>();

                if let Some(rest) = value.strip_prefix('=') {
                    value = rest.to_owned();
                }

                if value.is_empty() {
                    value = value_os("-u", iter);
                }

                args.in_unit = Some(unit("-u", &value));
                return;
            }

            option => {
                error(&format!("unknown option: `-{option}`"));
            }
        }

        i += 1;
    }
}

fn set_disk(args: &mut Args, attached: Option<&str>) {
    no_value("--disk-usage", attached);

    if args.disk_usage {
        error("duplicate option: `--disk-usage`");
    }

    if !args.paths.is_empty() {
        error("`--disk-usage` cannot be used with paths");
    }

    if args.metadata {
        error("`--disk-usage` cannot be used with `--metadata`");
    }

    if args.follow_symlinks {
        error("`--disk-usage` cannot be used with `--follow-symlinks`");
    }

    if args.binary {
        error("`--disk-usage` cannot be used with `--binary`");
    }

    if args.raw || args.b_flag {
        error("`--disk-usage` cannot be used with `--raw`");
    }

    if args.in_unit.is_some() {
        error("`--disk-usage` cannot be used with `--unit`");
    }

    if !args.excludes.is_empty() {
        error("`--disk-usage` cannot be used with `--exclude`");
    }

    if args.max_depth.is_some() {
        error("`--disk-usage` cannot be used with `--max-depth`");
    }

    args.disk_usage = true;
}

fn check_disk(args: &Args, option: &str) {
    if args.disk_usage {
        error(&format!(
            "option `{option}` cannot be used with `--disk-usage`"
        ));
    }
}

fn no_value(option: &str, value: Option<&str>) {
    if value.is_some() {
        error(&format!("option `{option}` does not accept a value"));
    }
}

fn value(
    option: &str,
    attached: Option<&str>,
    iter: &mut impl Iterator<Item = OsString>,
) -> String {
    match attached {
        Some(value) => value.to_owned(),
        None => value_os(option, iter),
    }
}

fn value_os(option: &str, iter: &mut impl Iterator<Item = OsString>) -> String {
    let value = match iter.next() {
        Some(value) => value,
        None => error(&format!("option `{option}` requires a value")),
    };

    match value.into_string() {
        Ok(value) => value,
        Err(_) => error(&format!("value for `{option}` must be valid UTF-8")),
    }
}

fn number(option: &str, value: &str) -> usize {
    match value.parse() {
        Ok(value) => value,
        Err(_) => error(&format!("invalid value for `{option}`: `{value}`")),
    }
}

fn unit(option: &str, value: &str) -> Unit {
    match value.parse() {
        Ok(unit) => unit,
        Err(_) => error(&format!("invalid value for `{option}`: `{value}`")),
    }
}

fn validate(args: &Args) {
    if args.b_flag && args.raw {
        error("same same");
    }

    let raw = args.raw || args.b_flag;

    if args.binary && raw {
        error("options `--binary` and `-r` cannot be used together");
    }

    if args.binary && args.in_unit.is_some() {
        error("options `--binary` and `-u`, pick one");
    }

    if raw && args.in_unit.is_some() {
        error("do you want raw bytes or only bytes?");
    }

    if args.metadata && args.follow_symlinks {
        error("options `-m` and `-L` cannot be used together");
    }

    if args.metadata && !args.excludes.is_empty() {
        error("option `-m` cannot be used with `--exclude`");
    }

    if args.metadata && args.max_depth.is_some() {
        error("option `-m` cannot be used with `--max-depth`");
    }
}

fn error(message: &str) -> ! {
    eprint!(
        "{}{}error{}",
        fsize_core::Color::bold(),
        fsize_core::Color::red(),
        fsize_core::Color::reset(),
    );

    eprintln!(": {message}");
    process::exit(2);
}

fn version() -> ! {
    println!("fsize {VERSION_NAME} {}", env!("CARGO_PKG_VERSION"));
    process::exit(0);
}

fn help() -> ! {
    println!("fsize");
    println!("    show file and directory sizes");
    println!();

    println!(
        "{}{}Usage:{}",
        fsize_core::Color::bold(),
        fsize_core::Color::green(),
        fsize_core::Color::reset()
    );

    println!(
        "    fsize [{}OPTIONS{}] [{}PATH{}]...",
        fsize_core::Color::yellow(),
        fsize_core::Color::reset(),
        fsize_core::Color::yellow(),
        fsize_core::Color::reset(),
    );

    println!(
        "    fsize {}-d{}",
        fsize_core::Color::dim(),
        fsize_core::Color::reset(),
    );

    println!();

    println!(
        "{}{}Options:{}",
        fsize_core::Color::bold(),
        fsize_core::Color::green(),
        fsize_core::Color::reset()
    );

    option("    -m, --metadata", "Show the size recorded in metadata");

    option("    -d, --disk-usage", "Show filesystem space");

    option("    -L, --follow-symlinks", "Follow symbolic links");

    option("    --binary", "Use binary units");

    option("    -r, -b, --raw, --byte", "Show bytes without formatting");

    argument("    -u, --unit", "UNIT", "Use a specific unit");

    argument(
        "    --exclude",
        "PATTERN",
        "Exclude matching files or directories",
    );

    argument("    --max-depth", "N", "Limit directory traversal depth");

    option(
        "    --json[=fmt]",
        "Output JSON (fmt: indented and colored)",
    );

    option("    -h, --help", "Show this help");

    option("    -V, --version", "Show version");

    println!();

    println!(
        "{}{}Examples:{}",
        fsize_core::Color::bold(),
        fsize_core::Color::green(),
        fsize_core::Color::reset()
    );

    example("fsize", "Show entries in the current directory");

    example("fsize Image/", "Show the total size of Image/");

    example("fsize image.jpg", "Show the size of image.jpg");

    example("fsize -m image.jpg", "Show the size recorded in metadata");

    example("fsize -d", "Show filesystem space");

    process::exit(0);
}

fn option(flag: &str, text: &str) {
    println!("{flag}");
    println!(
        "        {}{text}{}",
        fsize_core::Color::dim(),
        fsize_core::Color::reset(),
    );
}

fn argument(flag: &str, value: &str, text: &str) {
    println!(
        "{flag} {}{value}{}",
        fsize_core::Color::yellow(),
        fsize_core::Color::reset(),
    );

    println!(
        "            {}{text}{}",
        fsize_core::Color::dim(),
        fsize_core::Color::reset(),
    );
}

fn example(command: &str, text: &str) {
    println!("    {command}");
    println!(
        "        {}{text}{}",
        fsize_core::Color::dim(),
        fsize_core::Color::reset(),
    );
}
