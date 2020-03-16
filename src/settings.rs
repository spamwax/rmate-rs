use std::ffi::OsString;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rmate", about = "rmate ♥ Rust (TextMate & Sublime Text)")]
pub(crate) struct Settings {
    /// Connect to HOST. Use 'auto' to detect the host from SSH
    /// Defalts to localhost
    #[structopt(
        short = "H",
        long = "--host",
        env = "RMATE_HOST",
        default_value = "localhost"
    )]
    pub host: String,

    /// Port number to use for connection. Defalts to 52698
    #[structopt(short, long, env = "RMATE_PORT", default_value = "52698")]
    pub port: u16,

    /// Wait for file to be closed by TextMate/Sublime Text
    #[structopt(short, long)]
    pub wait: bool,

    /// Open even if file is not writable.
    /// This flag willl affect all files
    #[structopt(short, long)]
    pub force: bool,

    /// Verbose logging messages (can be repeated: -vvv)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// The display name shown in TextMate/Sublime Text
    #[structopt(short = "m", long = "name", parse(from_os_str), number_of_values = 1)]
    pub names: Vec<OsString>,

    #[structopt(parse(from_os_str), required(true))]
    pub files: Vec<OsString>,
}

// use std::collections::hash_map::HashMap;

#[derive(Debug)]
pub(crate) struct OpenedBuffer {
    pub(crate) canon_path: PathBuf,
    pub(crate) display_name: OsString,
    pub(crate) canwrite: bool,
    pub(crate) temp_file: File,
    pub(crate) size: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            host: "localhost".to_string(),
            port: 52698,
            wait: false,
            force: false,
            verbose: 0,
            names: vec![],
            files: vec![],
        }
    }
}
