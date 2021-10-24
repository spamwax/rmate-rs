use log::*;
use serde::Deserialize;
use std::ffi::OsString;
use std::fs::canonicalize;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;

pub(crate) const NO_TRIES_CREATE_BACKUP_FN: u8 = 5;
pub(crate) const RMATE_HOST: &str = "localhost";
pub(crate) const RMATE_PORT: u16 = 52698;

// program settings from command-line arguments and environment variables
#[derive(Debug, StructOpt)]
#[structopt(
    name = "rmate",
    author = " ",
    about = "rmate ♥ Rust (TextMate & Sublime Text)",
    settings(&[AppSettings::ColoredHelp])
)]
pub(crate) struct Settings {
    /// Connect to HOST. Use 'auto' to detect the host from SSH.
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

    /// Create a new file if the given file name does not exist
    #[structopt(short, long)]
    pub create: bool,

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

// Read host/settings from rmate.rc files
pub(crate) fn read_disk_settings() -> (String, u16) {
    trace!("Loading settings from rmate.rc files");
    let host_port = (self::RMATE_HOST.to_string(), self::RMATE_PORT);
    ["/etc/rmate.rc", "/usr/local/etc/rmate.rc", "~/.rmate.rc"]
        .iter()
        .inspect(|path| {
            trace!("Trying {}", path);
        })
        .map(|path| {
            if path.starts_with("~/") && dirs::home_dir().is_some() {
                canonicalize(dirs::home_dir().unwrap().join(&path[2..]))
            } else {
                canonicalize(path)
            }
        })
        .filter(|canon| canon.is_ok())
        .map(|canon| {
            let path = canon.unwrap();
            let fname = &Path::new(&path);
            (File::open(fname), path)
        })
        .inspect(|(file_result, path)| {
            if file_result.is_err() {
                trace!("  Cannot open {}", path.display());
            } else {
                trace!("  Found rc file at: {}", path.display());
            }
        })
        .filter(|(file_result, _)| file_result.is_ok())
        .map(|(fp, path)| {
            let buf_reader = BufReader::new(fp.unwrap());
            (serde_yaml::from_reader(buf_reader), path)
        })
        .inspect(
            |(s, path): &(Result<self::RcSettings, serde_yaml::Error>, PathBuf)| {
                if s.is_err() {
                    trace!("  Error parsing data in {}", path.display());
                    trace!("    {:?}", s.as_ref().unwrap_err());
                } else {
                    trace!(
                        "  Read disk settings-> {{ host: {:?}\tport: {:?} }}",
                        s.as_ref().unwrap().host.as_ref(),
                        s.as_ref().unwrap().port.as_ref(),
                    );
                }
            },
        )
        .filter(|(s, _)| s.is_ok())
        .map(|(s, _)| s.unwrap())
        .fold(host_port, |acc, item: self::RcSettings| {
            let (mut newhost, mut newport) = acc;
            if let Some(host) = item.host {
                newhost = host;
            }
            if let Some(port) = item.port {
                newport = port;
            }
            (newhost, newport)
        })
}
