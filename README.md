# fsize

The `fsize` utility is a command-line tool designed to calculate and display the total disk usage consumed by files or directories. It processes the specified files or folders and presents the resulting storage information in a user-friendly and easy-to-read format, making it simple for users to understand how much disk space is being used by their data.

__How to use it__:


Options:

```rust
-b, --binary
-r, --raw
-o, --byte
-i, --info
-m, --metadata
-u, --unit <UNIT>
-h, --help         Print help
-V, --version      Print version
```

Examples:

```rust
fsize file.txt             | 24 KB
fsize -b file.txt          | 20 KiB
fsize -o file.txt          | 160000
fsize file.txt -u MiB      | 0.02 MiB
fsize -i file.txt          | 24 KB f Jun 24 17:32
fsize -i /some/dir         | 1.2 GB d Jun 24 17:32
```
