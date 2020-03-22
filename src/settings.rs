use serde::Deserialize;
use std::ffi::OsString;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

pub(crate) const NO_TRIES_CREATE_BACKUP_FN: u8 = 5;
pub(crate) const RMATE_HOST: &'static str = "localhost";
pub(crate) const RMATE_PORT: u16 = 52698;

// program settings from command-line arguments and environment variables
#[derive(Debug, StructOpt)]
#[structopt(
    name = "rmate",
    author = "h@mid.fyi",
    about = "rmate ♥ Rust (TextMate & Sublime Text)"
)]
pub(crate) struct Settings {
    /// Connect to HOST. Use 'auto' to detect the host from SSH
    /// Defalts to localhost
    #[structopt(short = "H", long = "--host", env = "RMATE_HOST", name = "HOST")]
    pub host: Option<String>,

    /// Port number to use for connection. Defalts to 52698
    #[structopt(short, long, env = "RMATE_PORT", min_values = 1)]
    pub port: Option<u16>,

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

    /// Place caret on line [NUMBER] after loading file
    #[structopt(short, long, name = "LINE", number_of_values = 1)]
    pub lines: Vec<String>,

    /// Treat file as having [TYPE]
    #[structopt(short = "t", long = "type", name = "TYPE", number_of_values = 1)]
    pub filetypes: Vec<String>,

    #[structopt(parse(from_os_str), required(true))]
    pub files: Vec<OsString>,
}

// struct to hold info about each opened file
#[derive(Debug)]
pub(crate) struct OpenedBuffer {
    pub(crate) canon_path: PathBuf,
    pub(crate) display_name: OsString,
    pub(crate) line: String,
    pub(crate) filetype: Option<String>,
    pub(crate) canwrite: bool,
    pub(crate) temp_file: File,
    pub(crate) size: u64,
}

#[derive(Debug, Deserialize)]
pub struct RcSettings {
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) unixsocket: Option<String>,
}
