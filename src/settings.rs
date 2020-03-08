use std::ffi::OsString;

pub struct Settings {
    pub host: String,
    pub port: u16,
    pub wait: bool,
    pub force: bool,
    pub verbose: u8,
    pub names: Vec<OsString>,
    pub files: Vec<OsString>,
}

impl Settings {
    pub fn new(port: u16) -> Self {
        Settings {
            port: port,
            ..Default::default()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            host: "localhost".to_string(),
            port: 52696,
            wait: false,
            force: false,
            verbose: 0,
            names: vec![],
            files: vec![],
        }
    }
}
