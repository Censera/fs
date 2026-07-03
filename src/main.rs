use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;

use fsize::{compute_total_size, format_mtime, format_size, Color, FsizeError, Unit, WalkOutcome};

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

    #[arg(short = 'b', long = "binary")]
    binary: bool,

    #[arg(short = 'r', long = "raw")]
    raw: bool,

    #[arg(short = 'o', long = "byte")]
    byte: bool,

    #[arg(short = 'i', long = "info")]
    info: bool,

    #[arg(short = 'm', long = "metadata")]
    metadata: bool,

    #[arg(short = 'u', long = "unit", value_name = "UNIT")]
    in_unit: Option<Unit>,
}

fn main() {
    let args = Args::parse();

    let raw = args.raw || args.byte;
    let mut exit_code = 0;

    for path in &args.paths {
        let outcome: Result<WalkOutcome, FsizeError> = if args.metadata {
            fs::symlink_metadata(path)
                .map(|m| WalkOutcome {
                    total: m.len() as u128,
                    warnings: Vec::new(),
                })
                .map_err(|e| FsizeError::Io {
                    path: path.to_owned(),
                    source: e,
                })
        } else {
            compute_total_size(path)
        };

        match outcome {
            Ok(WalkOutcome { total, warnings }) => {
                for w in &warnings {
                    eprintln!("{}[WARNING]{} {}", Color::yellow(), Color::reset(), w);
                }
                if !warnings.is_empty() {
                    exit_code = 1;
                }

                let size_str = if raw {
                    total.to_string()
                } else {
                    format_size(total, args.in_unit, args.binary)
                };

                let mut output = size_str;

                if args.info {
                    match fs::symlink_metadata(path) {
                        Ok(meta) => {
                            let ft = meta.file_type();
                            let type_char = if ft.is_dir() {
                                'D'
                            } else if ft.is_symlink() {
                                'L'
                            } else {
                                'F'
                            };
                            let mut extra = String::new();
                            extra.push(type_char);
                            extra.push(' ');
                            if let Ok(mtime) = meta.modified() {
                                extra.push_str(&format_mtime(mtime));
                            }
                            output.push(' ');
                            output.push_str(&extra);
                        }
                        Err(e) => {
                            eprintln!(
                                "{}[WARNING]{} Cannot read metadata for `{}`: {}",
                                Color::yellow(),
                                Color::reset(),
                                path.display(),
                                e
                            );
                            exit_code = 1;
                        }
                    }
                }

                if args.paths.len() > 1 {
                    println!("{}\t{}", output, path.display());
                } else {
                    println!("{}", output);
                }
            }
            Err(e) => {
                eprintln!("{}[ERROR]{} {}", Color::red(), Color::reset(), e);
                exit_code = 1;
            }
        }
    }

    process::exit(exit_code);
}
