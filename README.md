# kazoe

Fast `wc` replacement.

Command: `kz`

## Install

```bash
cargo install kazoe
```

Or from source:
```bash
git clone https://github.com/ConeDjordjic/kazoe
cd kazoe
cargo install --path .
```

## Performance

1GB text file benchmarks:

```
Word count:     21x faster (48ms vs 1.0s)
All counts:     16x faster (63ms vs 1.0s)
Pattern match:  90x faster (32ms vs 2.9s)
Line count:     1.7x faster (32ms vs 56ms)
Multi-file:     23x faster (86ms vs 2.0s)
```

- Files < 512KB: sequential, similar to wc
- Files > 1GB: 15-90x faster

## Usage

```bash
kz file.txt              # lines, words, bytes
kz -l file.txt           # lines
kz -w file.txt           # words
kz -c file.txt           # bytes
kz -m file.txt           # characters (UTF-8)
kz -L file.txt           # max line length
kz -lwc file.txt         # combine flags
kz -w *.txt              # multiple files
cat file.txt | kz -w     # stdin
kz --pattern "foo" file  # count pattern
```

## Options

```
-l, --lines              line count
-w, --words              word count
-c, --bytes              byte count
-m, --chars              character count (UTF-8)
-L, --max-line-length    longest line
-r, --recursive          recurse directories
-v, --verbose            show warnings
--unique                 unique word count
--pattern <PAT>          count pattern occurrences
--stats                  show statistics (mean, median, std dev)
--histogram              line length distribution
--json                   JSON output
--timing                 show processing time
--progress               show progress
--fast                   skip UTF-8 validation
--code                   skip comments (// /* # -- """)
--markdown               skip code blocks
--exclude <PAT>          exclude files matching pattern
--encoding <ENC>         force encoding (auto-detects otherwise)
--files0-from <FILE>     read null-terminated filenames
--generate-completion    shell completions (bash/zsh/fish/powershell)
```

## Examples

```bash
# Recursive with exclusions
kz -r --exclude "*.min.js" --exclude "node_modules/*" src/

# Statistics
kz --stats file.txt

# JSON output with timing
kz --json --timing file.txt

# Code lines only (skip comments)
kz --code -l src/*.rs

# Markdown text only (skip code blocks)
kz --markdown -w README.md

# Force encoding
kz --encoding iso-8859-1 legacy.txt

# Progress for large operations
kz --progress -r ~/projects/
```

## Shell Completions

```bash
kz --generate-completion bash > ~/.local/share/bash-completion/completions/kz
kz --generate-completion zsh > ~/.zfunc/_kz
kz --generate-completion fish > ~/.config/fish/completions/kz.fish
```

## Implementation

- Parallel processing (Rayon, 1MB chunks)
- Memory-mapped I/O
- SIMD pattern matching (memchr)
- UTF-8 aware with proper chunk boundaries
- Unicode whitespace detection
- CRLF normalization
- Encoding detection and conversion
- Binary file detection

## License

MIT
