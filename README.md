[![CircleCI](https://img.shields.io/circleci/build/gh/listboss/rmate-rust?logo=circleci&style=for-the-badge)](https://circleci.com/gh/listboss/rmate-rust)
# rmate ♥ Rust

fast, reliable implementation of **rmate** in Rust.

Pre-complied binaries for multiple platforms are available in [Releases page](https://github.com/azhinu/rmate-rust/releases).

### Features

- Support all options and rmate.rc settings described in [Ruby implementation](https://github.com/textmate/rmate).
- Create backups of local files to avoid corrupting your files if any io/network operations go wrong.
- Verbose logging of operation (use `-v` one or more times). See [![the screencat](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26.svg)](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26).

### Build from sources

You can build **rmate-rs** with docker using `build.sh` script. Just run:
```shell
./build.sh <os target you need>
```

*:information_source: tested only with `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl` targets.*

### Difference with spamwax/rmate-rs

- Ask to create file if not exist.
- `--nocreate` flag to avoid file creating.

### Changelog:
- 20.22.04.12:
  - Now asking before creating non-existing file.
  - Added build script.
  - Added build options to `Cargo.toml` allowing reduce binary size.
  - Updated `.gitignore`.
- 2022.04.06:
  - Inverted `--create` flag.
  - Updated `.gitignore`.
  - Removed unused CI config.
