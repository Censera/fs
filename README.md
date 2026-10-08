[![Build](https://github.com/Censera/fsize/actions/workflows/release.yml/badge.svg)](https://github.com/Censera/fsize/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/fsize-cli.svg)](https://crates.io/crates/fsize-cli)
[![License](https://img.shields.io/crates/l/fsize-cli.svg)](LICENSE)

Measure file and directory sizes from the command line. Walks paths in parallel and reports human-readable sizes by default, with flags for raw bytes, forced units, JSON output, and mount-level disk usage.

The repository is a Cargo workspace: `core` (`fsize-core`) holds the size-computation and formatting logic; `cli` (`fsize-cli`) is the command-line tool built on top of it.

## Install

```r
cargo install fsize-cli
```

The crate is published as `fsize-cli` because `fsize` is taken on crates.io by an unrelated type alias. The installed binary is still named `fsize`.

Pre-built binaries for Linux (x86_64, aarch64, armv7, riscv64), macOS, and Windows are attached to each [release](https://github.com/Censera/fsize/releases).

**From source** (requires rustc 1.85+):

```r
cargo build --release
```

## Usage

```ts
fsize [OPTIONS] [PATH]...

Options:
      --binary            Base-2 (1024) units instead of base-10 (1000)
  -r, -b, --raw, --byte   Exact byte count, no unit conversion
  -m, --metadata          Entry's own size via stat(), no recursive walk
  -d, --disk-usage        Mount-level total/used/free/available of the root filesystem
  -u, --unit <UNIT>       Force a unit: KB, MiB, GB, etc.
      --exclude <PATTERN> Skip entries matching PATTERN (glob, repeatable)
      --max-depth <N>     Limit directory recursion to N levels
  -L, --follow-symlinks   Follow symlinks while walking
      --json[=fmt]        JSON output; =fmt is indented and color-coded
  -h, --help
  -V, --version
```

`-b` is the same flag as `-r`; passing both is an error ("same same"). `--raw` and `--unit`/`--binary` are mutually exclusive, and `--disk-usage` only combines with `--json`.

With several paths (or when listing the current directory), each entry is printed as soon as it has been measured, followed by a `total` row. `--json` streams the same way: one entry per measurement, then the totals.

## What `-m` and `-d` do

`-m` reports the size of the path entry itself, the same number `stat` gives you. `-d` reports the size of the filesystem the path lives on. Neither is the recursive content size that a plain `fsize PATH` gives you.

## Examples

```ts
fsize file.txt                   24 KB
fsize --binary file.txt          24 KiB
fsize -r file.txt                24576
fsize -b file.txt                24576
fsize -u MiB file.txt            0.02 MiB

fsize file1.txt file2.txt
file1.txt    12 B
file2.txt    50 KB
total        50.01 KB

fsize --exclude 'target' --max-depth 3 .

fsize -d

fsize --json some-dir/
fsize --json=fmt some-dir/
```

`--json` prints a compact object per path (`{"path","bytes","formatted"}`), or `{"entries":[...],"total_bytes","total_formatted"}` for several. Byte counts are strings so they survive values above 2^53. `--json=fmt` is the same data, indented and color-coded; colors follow `NO_COLOR` / `FORCE_COLOR` and are off when the output is not a terminal.

## Benchmarks

Measured against GNU `du` and `diskus` on a ~77 GB `/home`, page cache warm. Not yet averaged across multiple runs.

**Binary size (stripped):**

```ts
fsize 0.1.1    826.42 KB
fsize 0.2.0    919.07 KB
diskus         932.42 KB
GNU du         1.6 MB
```

**Wall-clock time (real / user / sys):**

```ts
fsize 0.1.1    6.918s   3.271s   7.617s
fsize 0.2.0    4.805s   4.874s   7.785s
diskus         6.816s   7.760s  11.956s
GNU du         5.641s   1.673s   3.819s
```

fsize 0.2.0 has the lowest wall-clock time of the four. The high user+sys total relative to wall clock is consistent with parallel directory walking across multiple threads.

These numbers were measured on 0.1.1 and 0.2.0, before the 1.0 rewrite, and have not been re-run.

At the time fsize reported ~6.16 GB more than `du` and `diskus` on the same tree, attributed to hardlinks being counted once per directory entry. Hardlinked files are now counted once, by inode (Unix only; on Windows every directory entry is counted). The gap was not re-measured on the original tree.

## Contributing

Bug reports and patches go through [GitHub Issues](https://github.com/Censera/fsize/issues).

## License

[MIT](LICENSE)
