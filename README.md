[![Build](https://github.com/Censera/fsize/actions/workflows/release.yml/badge.svg)](https://github.com/Censera/fsize/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/fsize-cli.svg)](https://crates.io/crates/fsize-cli)
[![License](https://img.shields.io/crates/l/fsize-cli.svg)](LICENSE)

Measure file and directory sizes from the command line. Walks paths in parallel and reports human-readable sizes by default, with flags for raw bytes, forced units, JSON output, and mount-level disk usage.

The repository is a Cargo workspace: `fsize-core` holds the size-computation and formatting logic; `fsize` is the CLI built on top of it.

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

**Nix:**

```nix
nix build
```

**Arch Linux:** a `PKGBUILD` is included in the repository root.

## Usage

```ts
fsize [OPTIONS] [PATH]...

Options:
  -b, --binary            Base-2 (1024) units instead of base-10 (1000)
  -r, --raw, --byte       Exact byte count, no unit conversion
  -i, --info              Entry type (F/D/L) and last-modified time
  -m, --metadata          Entry's own size via stat(), no recursive walk
  -d, --disk-usage        Mount-level total/used/available for the filesystem containing PATH
  -u, --unit <UNIT>       Force a unit: KB, MiB, GB, etc.
      --exclude <PATTERN> Skip entries matching PATTERN (glob, repeatable)
      --max-depth <N>     Limit directory recursion to N levels
  -L, --follow-symlinks   Follow symlinks while walking
      --json              JSON output
  -h, --help
  -V, --version
```

`--raw` and `--unit`/`--binary` are mutually exclusive, as are `--metadata` and `--disk-usage`.

## What `-m` and `-d` do

`-m` reports the size of the path entry itself, the same number `stat` gives you. `-d` reports the size of the filesystem the path lives on. Neither is the recursive content size that a plain `fsize PATH` gives you.

## Examples

```ts
fsize file.txt                   24 KB
fsize -b file.txt                20 KiB
fsize -r file.txt                24576
fsize -u MiB file.txt            0.02 MiB
fsize -i file.txt                24 KB  F  Jun 24 17:32 UTC
fsize -i some-dir/               1.2 GB D  Jun 24 17:32 UTC

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

fsize reports ~6.16 GB more than `du` and `diskus` on the same tree. The likely cause is hardlink double-counting: `du` and `diskus` deduplicate by inode; fsize sums every directory entry without checking inode identity. This needs verification against a directory with known hardlinks before the byte counts can be trusted over `du`.

## Contributing

Bug reports and patches go through [GitHub Issues](https://github.com/Censera/fsize/issues).

## License

[MIT](LICENSE)
