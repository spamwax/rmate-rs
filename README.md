[![CircleCI](https://img.shields.io/circleci/build/gh/listboss/rmate-rust?logo=circleci&style=for-the-badge)](https://circleci.com/gh/listboss/rmate-rust)
# rmate ♥ Rust

fast, reliable implementation of **rmate** in Rust.

Pre-complied binaries for multiple platforms are available in [Releases page](https://github.com/listboss/rmate-rust/releases).

### Features

- Support all options and rmate.rc settings described in [Ruby implementation](https://github.com/textmate/rmate).
- Create backups of local files to avoid corrupting your files if any io/network operations go wrong.
- Verbose logging of operation (use `-v` one or more times). See [![the screencat](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26.svg)](https://asciinema.org/a/fqgvpm9yPdDFAZ11f8uY1DF26).
