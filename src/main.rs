use fork::{fork, Fork};
use log::*;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Error, ErrorKind};

mod file_handler;
mod remote_editor;
mod settings;
use settings::OpenedBuffer;
use settings::Settings;
use structopt::StructOpt;

fn main() -> Result<(), String> {
    // Read settings from cmd line arguments
    let mut settings = Settings::from_args();

    let level;
    match env::var("RUST_LOG") {
        Err(_) => {
            match settings.verbose {
                0 => level = "warn",
                1 => level = "info",
                2 => level = "debug",
                _ => level = "trace",
            }
            env::set_var("RUST_LOG", level);
        }
        _ => {}
    }
    env_logger::init();

    // Set host/port if user didn't specify in arguments and
    // we found them in one of rmate.rc files. Otherwise use
    // default values.
    let disk_settings = settings::read_disk_settings();
    settings.host.get_or_insert(disk_settings.0);
    settings.port.get_or_insert(disk_settings.1);

    // if --host auto is set in cmd line arguments, we try to find the host address from
    // SSH_CONNECTION
    if settings.host.as_ref().unwrap() == "auto" {
        trace!("Finding host automatically from SSH_CONNECTION");
        let auto_host = env::var("SSH_CONNECTION").map_or("localhost".to_string(), |conn| {
            // iterator returned by split() always returns at least one item so unwrap() is safe
            conn.split(' ').take(1).next().unwrap().to_string()
        });
        trace!("  from SSH_CONNECTION: {}", auto_host);
        settings.host.as_mut().map(|host| *host = auto_host);
    }

    trace!("rmate settings: {:#?}", settings);

    // Check & connect to socket
    let socket = remote_editor::connect_to_editor(&settings).map_err(|e| e.to_string())?;
    // Populate internal data about requested files in OpenedBuffer structure
    let buffers = file_handler::get_requested_buffers(&settings)?;
    // Send the files to remote editor
    // let buffers = remote_editor::open_file_in_remote(&socket, buffers)?;
    remote_editor::open_file_in_remote(&socket, &buffers)?;

    // If needed, fork so we yield the control back to terminal
    if !settings.wait && run_fork()? {
        debug!("Successfully forked!");
        return Ok(());
    }

    // Wait for save/close instructions from remote and handle them
    handle_remote(socket, buffers).map_err(|e| e.to_string())?;

    Ok(())
}

// On successfull fork(), parent return true and child returns false.
fn run_fork() -> Result<bool, String> {
    match fork() {
        Ok(Fork::Parent(child)) => {
            trace!("Parent process created a child: {}", child);
            return Ok(true);
        }
        Ok(Fork::Child) => {
            trace!("Child says: I AM BORN!");
            return Ok(false);
        }
        Err(e) => {
            error!("{}", e.to_string());
            return Err(format!("OS Error no. {}", e));
        }
    }
}

fn handle_remote(
    socket: socket2::Socket,
    mut opened_buffers: HashMap<String, OpenedBuffer>,
) -> Result<(), std::io::Error> {
    let mut total = 0;
    debug!("Waiting for editor's instructions...");

    let mut myline = String::with_capacity(128);
    let bsize = socket.recv_buffer_size()? * 2;
    trace!("socket recv size: {}", bsize);

    let mut buffer_reader = BufReader::with_capacity(bsize, &socket);
    // Wait for commands from remote app
    // let mut line = Vec::<u8>::with_capacity(64);
    while buffer_reader.read_line(&mut myline)? != 0 {
        debug!(
            "=== Received line from editor (trimmed): >>{}<<",
            myline.trim()
        );
        match myline.trim() {
            // close the buffer for a file
            "close" => {
                trace!("--> About to close_buffer()");
                myline.clear();
                remote_editor::close_buffer(&mut opened_buffers, &mut buffer_reader);
            }
            // save the buffer to a file
            "save" => {
                trace!("--> About to call write_to_disk()");
                myline.clear();
                match file_handler::write_to_disk(&mut opened_buffers, &mut buffer_reader, bsize) {
                    Ok(n) => total += n,
                    Err(e) => error!("Couldn't save: {}", e.to_string()),
                }
            }
            _ => {
                if myline.trim() == "" {
                    trace!("<-- Recvd empty line from editor");
                    continue;
                } else {
                    error!("***===*** Unrecognized shit: {:?}", myline.trim());
                    return Err(Error::new(ErrorKind::Other, "unrecognized shit"));
                }
            }
        }
    }
    trace!("Cumulative total bytes saved: {}", total);
    Ok(())
}
