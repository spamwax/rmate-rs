[![Test & Release binaries](https://github.com/spamwax/rmate-rs/actions/workflows/release.yml/badge.svg)](https://github.com/spamwax/rmate-rs/actions/workflows/release.yml)

# rmate ♥ Rust

fast, reliable implementation of **rmate** in Rust.

Pre-complied binaries for following platforms are available in [Releases page](https://github.com/listboss/rmate-rust/releases).

| Platform |                                                |
|----------|------------------------------------------------|
| macOS    | x86_64, aarch64                                |
| Linux    | x86_64, i686, aarch64, armv7                   |
| FreeBSD  | x86_64                                         |
| Android  | x86_64, i686, aarch64, armv7, arm, thumbv7neon |
| Illumos  | x86_64                                         |

### Features

- Support all options and rmate.rc settings described in [Ruby implementation](https://github.com/textmate/rmate).
- Create backups of local files to avoid corrupting your files if any io/network operations go wrong.
- Verbose logging of operation (use `-v` one or more times).

### Demo
[![the screencat](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26.svg)](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26).
