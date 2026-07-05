# fsize

[![Build Status](https://github.com/Censera/fsize/actions/workflows/release.yml/badge.svg)](https://github.com/Censera/fsize/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/fsize-cli.svg)](https://crates.io/crates/fsize-cli)
[![License](https://img.shields.io/crates/l/fsize-cli.svg)](LICENSE)

fsize computes file and directory sizes from the command line. It walks a
path in parallel and sums file sizes, or reports mount level disk usage
(total, used, available) for the filesystem containing a path. Output is
nice readable text by default, raw byte and JSON output are available for
scripts.

The repository is a Cargo workspace with two crates: `fsize-core`, the
size-computation and formatting logic, and `fsize`, the CLI built on top
of it.

## Building

Requires a Rust toolchain supporting the 2024 edition (rustc 1.85+).

```ts
cargo build --release
```

A Nix flake is provided and builds the same targets the release CI does
(Linux x86_64/aarch64/armv7/riscv64, macOS, Windows):

```ts
nix build
```

To install from crates.io once published:

```ts
cargo install fsize-cli
```

`fsize` itself is already taken on crates.io by an unrelated crate (a type
alias for the pointer-sized float type), so the package is published as
`fsize-cli`; the installed binary is still named `fsize`. Confirm the name
is still free before publishing.

Pre-built binaries are attached to each [release](https://github.com/Censera/fsize/releases).

## Usage

```ts
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
-V, --version
```

`--raw`/`--byte` and `--unit`/`--binary` are mutually exclusive, as are
`--metadata` and `--disk-usage`. Combining them is a usage error (exit
code 2).

Note that `-m` reports the size of PATH itself (like `stat`), and `-d`
reports the size of the filesystem PATH lives on. Neither is the
recursive content size that plain `fsize PATH` gives you.

### Examples

```rust
fsize file.txt                  24 KB
fsize -b file.txt               20 KiB
fsize -r file.txt               24576
fsize -u MiB file.txt           0.02 MiB
fsize -i file.txt               24 KB F Jun 24 17:32 UTC
fsize -i some-dir/              1.2 GB D Jun 24 17:32 UTC

fsize file1.txt file2.txt
12 B      file1.txt
50 KB     file2.txt
50.01 KB  total

fsize --exclude 'target' --max-depth 3 .

fsize -d /
/   total 512.00 GB   used 210.34 GB   available 301.66 GB   (41.1% used)

fsize --json some-dir/
```

## Benchmarks

Measured against GNU du and diskus on a ~77 GB /home directory, page
cache warm, single run each. Not yet averaged over multiple runs; treat
as indicative.

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

fsize 0.2.0 has the lowest wall-clock time of the four, including below
single-threaded GNU du despite du's lower total CPU time. user+sys for
fsize is about 12.7s against a 4.8s wall clock, versus du's roughly 1:1
ratio, consistent with du running single-threaded and fsize running its
walk across multiple threads.

Reported size, in bytes:

```ts
fsize 0.1.1    77,661,965,054
fsize 0.2.0    77,662,366,082
diskus         71,502,820,852
GNU du         71,502,838,897
```

fsize reports about 6.16 GB more than diskus and du, consistently across
both fsize versions measured. This is not evidence that fsize is more
accurate. The size of the offset and its stability across versions points
to hardlink double-counting: du and diskus both deduplicate by inode, so
a file linked from multiple paths is counted once; fsize's current walk
sums every directory entry independently, without checking inode
identity. This needs to be verified against a directory with known
hardlinks, and fixed if confirmed, before the byte counts can be trusted
over du's.

## Contributing

Bug reports and patches go through
[GitHub Issues](https://github.com/Censera/fsize/issues).
