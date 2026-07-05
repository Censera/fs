# fsize

[![Build Status](https://github.com/Censera/fsize/actions/workflows/release.yml/badge.svg)](https://github.com/Censera/fsize/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/fsize-cli.svg)](https://crates.io/crates/fsize-cli)
[![License](https://img.shields.io/crates/l/fsize-cli.svg)](LICENSE)

<<<<<<< HEAD
fsize computes file and directory sizes from the command line. It walks a path in parallel and sums file sizes, or reports mount-level disk usage (total, used, available) for the filesystem containing a path. Default output is human-readable text. Raw byte and JSON outputs are available for scripts.
=======
fsize computes file and directory sizes from the command line. It walks a path in parallel and sums file sizes, or reports mount-level disk usage (total, used, available) for the filesystem containing a path. Default output is nice, readable text. Raw byte and JSON outputs are available for scripts.
>>>>>>> 40786aa02ef3f8aab143bb0d89b1947d178c5f79

The repository is a Cargo workspace containing two crates: `fsize-core` (the size-computation and formatting logic) and `fsize` (the CLI binary built on top of it).

## Building

Requires a Rust toolchain supporting the 2024 edition (rustc 1.85+).

```ts
cargo build --release

```

A Nix flake is provided to build targets corresponding to the release CI (Linux x86_64/aarch64/armv7/riscv64, macOS, Windows):

```ts
nix build

```

To install from crates.io:

```ts
cargo install fsize-cli

```

The package is published as `fsize-cli` because the name `fsize` is already taken on crates.io by an unrelated crate. The installed binary remains named `fsize`.

Pre-built binaries are attached to each [release](https://github.com/Censera/fsize/releases).

## Usage

```ts
<<<<<<< HEAD
-b, --binary          base-2 (1024) units instead of base-10 (1000)
-r, --raw, --byte       exact byte count, no unit conversion
-i, --info               entry type (F/D/L) and last-modified time
-m, --metadata           entry's own size via stat(), no recursive walk
-d, --disk-usage         mount-level total/used/available for the
filesystem containing PATH
-u, --unit <UNIT>        force a unit, e.g. KB, MiB, GB
--exclude <PATTERN>  skip entries matching PATTERN (glob, repeatable)
--max-depth <N>      limit recursion to N directories
-L, --follow-symlinks    follow symlinks while walking
--json               JSON output
h,  --help
=======
-b, --binary              base-2 (1024) units instead of base-10 (1000)
-r, --raw, --byte         exact byte count, no unit conversion
-i, --info                entry type (F/D/L) and last-modified time
-m, --metadata            entry's own size via stat(), no recursive walk
-d, --disk-usage          mount-level total/used/available for the
                          filesystem containing PATH
-u, --unit <UNIT>         force a unit, e.g. KB, MiB, GB
    --exclude <PATTERN>   skip entries matching PATTERN (glob, repeatable)
    --max-depth <N>       limit recursion to N directories
-L, --follow-symlinks     follow symlinks while walking
    --json                JSON output
-h, --help
>>>>>>> 40786aa02ef3f8aab143bb0d89b1947d178c5f79
-V, --version

```

`--raw`/`--byte` and `--unit`/`--binary` are mutually exclusive. `--metadata` and `--disk-usage` are mutually exclusive. Combining exclusive flags causes a usage error (exit code 2).

The `-m` flag reports the size of PATH itself via `stat`. The `-d` flag reports the size of the filesystem containing PATH. Neither flag performs the recursive content walk executed by plain `fsize PATH`.

### Examples

```ts
fsize file.txt               |   24 KB
fsize -b file.txt            |   20 KiB
fsize -r file.txt            |   24576
fsize -u MiB file.txt        |   0.02 MiB
fsize -i file.txt            |   24 KB F Jun 24 17:32 UTC
fsize -i some-dir/           |   1.2 GB D Jun 24 17:32 UTC
```

```ts
fsize file1.txt file2.txt
12 B      file1.txt
50 KB     file2.txt
50.01 KB  total
```

```ts
fsize --exclude 'target' --max-depth 3 .
```

```ts
fsize -d /
/   total 512.00 GB   used 210.34 GB   available 301.66 GB   (41.1% used)
```

```ts
fsize --json some-dir/

```

## Benchmarks

Measured against GNU du and diskus on a ~77 GB /home directory, page cache warm, single run.

Stripped binary size:

```ts
fsize 0.1.1    826.42 KB
fsize 0.2.0    919.07 KB
diskus         932.42 KB
GNU du         1.6 MB

```

Wall-clock time (real/user/sys):

```ts
fsize 0.1.1    6.918s   3.271s   7.617s
fsize 0.2.0    4.805s   4.874s   7.785s
diskus         6.816s   7.760s  11.956s
GNU du         5.641s   1.673s   3.819s

```

<<<<<<< HEAD
The user+sys time for fsize 0.2.0 is 12.7s against a 4.8s wall clock, reflecting parallel directory traversal across multiple threads. GNU du runs single-threaded with a roughly 1:1 ratio.
=======
The user + sys time for fsize 0.2.0 is 12.7s against a 4.8s wall clock, reflecting parallel directory traversal across multiple threads. GNU du runs single-threaded with a 1:1 ratio.
>>>>>>> 40786aa02ef3f8aab143bb0d89b1947d178c5f79

Reported size, in bytes:

```ts
fsize 0.1.1    77,661,965,054
fsize 0.2.0    77,662,366,082
diskus         71,502,820,852
GNU du         71,502,838,897

```

<<<<<<< HEAD
fsize reports approximately 6.16 GB more than diskus and du. du and diskus deduplicate by inode to count hard-linked files once. fsize sums directory entries independently without checking inode identity, causing hard-link double-counting.
=======
fsize reports approximately 6.16 GB more than diskus and du. du and diskus deduplicate by inode to count hard linked files once. fsize sums directory entries independently without checking inode identity, causing hardlink double counting.
>>>>>>> 40786aa02ef3f8aab143bb0d89b1947d178c5f79

## Contributing

Bug reports and patches go through [GitHub Issues](https://github.com/Censera/fsize/issues).
