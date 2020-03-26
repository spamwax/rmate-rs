use fork::{fork, Fork};
use log::*;
use socket2::{Domain, Type};
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::{canonicalize, metadata};
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, SeekFrom, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{fs, io};

// TODO: use 'group' feature of clap/structopt to parse: -m name1 namefile1 file1 file2 -m name2 namefile2 file3 <15-03-20, hamid> //
// TODO: Can we convert the fork() error number to a proper io::Error? <18-03-20, hamid> //
// TODO: refactor setting related files to settings.rs <20-03-20, hamid> //
// TODO: implement host=auto from SSH_CONNECTIONS <20-03-20, hamid> //

mod settings;
use settings::OpenedBuffer;
use settings::Settings;
use structopt::StructOpt;

fn main() -> Result<(), String> {
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
    let disk_settings = read_disk_settings();
    settings.host.get_or_insert(disk_settings.0);
    settings.port.get_or_insert(disk_settings.1);

    trace!("rmate settings: {:#?}", settings);

    // Check & connect to socket
    let socket = connect_to_editor(&settings).map_err(|e| e.to_string())?;
    // Populate internal data about files in OpenedBuffer structure
    let buffers = get_opened_buffers(&settings)?;
    // Send the files to remote editor
    let buffers = open_file_in_remote(&socket, buffers)?;

    // If needed, fork so we yield the control back to terminal
    if !settings.wait && run_fork()? {
        debug!("Successfully forked!");
        return Ok(());
    }

    // Wait for save/close instructions from remote and handle them
    handle_remote(socket, buffers).map_err(|e| e.to_string())?;

    Ok(())
}

// Read host/settings from rmate.rc files
fn read_disk_settings() -> (String, u16) {
    trace!("Loading settings from rmate.rc files");
    let host_port = (settings::RMATE_HOST.to_string(), settings::RMATE_PORT);
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
            |(s, path): &(Result<settings::RcSettings, serde_yaml::Error>, PathBuf)| {
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
        .fold(host_port, |acc, item: settings::RcSettings| {
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

fn connect_to_editor(settings: &Settings) -> Result<socket2::Socket, std::io::Error> {
    let socket = socket2::Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();

    debug!("Host: {}", settings.host.as_ref().unwrap());
    let host = settings.host.as_ref().unwrap();
    let addr_srv = if host == "localhost" {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    } else {
        settings
            .host
            .as_ref()
            .unwrap()
            .parse::<IpAddr>()
            .map_err(|e| Error::new(ErrorKind::AddrNotAvailable, e.to_string()))?
    };
    let port = settings.port.unwrap();
    let addr_srv = std::net::SocketAddr::new(addr_srv, port).into();

    debug!("About to connect to {:?}", addr_srv);
    socket.connect(&addr_srv)?;
    trace!(
        "Socket details: \n\tmy address: {:?}\n\tremote address {:?}",
        socket.local_addr()?,
        socket.peer_addr()?
    );
    Ok(socket)
}

fn get_opened_buffers(settings: &Settings) -> Result<HashMap<String, OpenedBuffer>, String> {
    let mut buffers = HashMap::new();

    // Iterate over all files and create bookkeeping info
    for (idx, file) in settings.files.iter().enumerate() {
        let filename_canon = canonicalize(file).map_err(|e| e.to_string())?;

        let file_name_string;
        if settings.names.len() > idx {
            file_name_string = settings.names[idx].clone();
        } else {
            file_name_string = filename_canon
                .file_name()
                .ok_or("no valid file name found in input argument".to_string())?
                .to_os_string();
        }

        let mut line = String::with_capacity(128);
        if idx < settings.lines.len() {
            line = settings.lines[idx].clone();
        }
        let filetype: Option<String> = if idx < settings.filetypes.len() {
            Some(settings.filetypes[idx].clone())
        } else {
            None
        };

        let md = metadata(&filename_canon).map_err(|e| e.to_string())?;
        if md.is_dir() {
            return Err("openning directory not supported".to_string());
        }
        let canwrite = is_writable(&filename_canon, &md);
        // Show a warning even though user has used the --force flag.
        if !canwrite && settings.force {
            warn!("{:?} is readonly!", filename_canon);
        }
        if !(canwrite || settings.force) {
            return Err(format!(
                "File {} is read-only, use -f/--force to open it anyway",
                file_name_string.to_string_lossy()
            ));
        }

        let filesize = md.len();
        let rand_temp_file = tempfile::tempfile().map_err(|e| e.to_string())?;

        let mut hasher = DefaultHasher::new();
        filename_canon.hash(&mut hasher);
        let hashed_fn = hasher.finish();
        trace!("hashed_fn (token): {:x}", hashed_fn);
        if let Some(v) = buffers.insert(
            hashed_fn.to_string(),
            OpenedBuffer {
                canon_path: filename_canon,
                display_name: file_name_string.clone(),
                line: line,
                filetype: filetype,
                canwrite: canwrite,
                temp_file: rand_temp_file,
                size: filesize,
            },
        ) {
            warn!(
                "You are trying to open same files multiple time: {}",
                v.canon_path.to_string_lossy().as_ref()
            );
        };
    }
    trace!("All opened buffers:\n{:#?}", &buffers);
    Ok(buffers)
}

fn open_file_in_remote(
    socket: &socket2::Socket,
    buffers: HashMap<String, OpenedBuffer>,
) -> Result<HashMap<String, OpenedBuffer>, String> {
    let bsize = socket.recv_buffer_size().map_err(|e| e.to_string())?;
    trace!("Socket recv buffer: {}", bsize);
    let bsize = socket.send_buffer_size().map_err(|e| e.to_string())? * 2;
    trace!("Socket send buffer: {}", bsize);

    let host_name = gethostname::gethostname();
    debug!("Hostname: {:?}", host_name);
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, socket);
        for (token, opened_buffer) in buffers.iter() {
            // For each buffer get the header values:
            // - display-name
            // - real-path
            // - selection/line
            // - file-type (optional)
            let mut total = 0usize;
            let header = format!(
                concat!(
                    "open\ndisplay-name: {}:{}\n",
                    "real-path: {}\n",
                    "selection: {}\n",
                    "data-on-save: yes\nre-activate: yes\n",
                    "token: {}\n",
                ),
                host_name.to_string_lossy(),
                opened_buffer.display_name.to_string_lossy(),
                opened_buffer.canon_path.to_string_lossy(),
                opened_buffer.line,
                token
            );
            trace!("header: {}", header);
            write!(&mut buf_writer, "{}", header).map_err(|e| e.to_string())?;

            if let Some(filetype) = &opened_buffer.filetype {
                write!(&mut buf_writer, "file-type: {}\n", filetype).map_err(|e| e.to_string())?;
                debug!("file-type: {}", filetype);
            }
            write!(&mut buf_writer, "data: {}\n", opened_buffer.size).map_err(|e| e.to_string())?;

            // Read file from disk and send it over the socket
            let fp = File::open(&opened_buffer.canon_path).map_err(|e| e.to_string())?;
            let mut buf_reader = BufReader::with_capacity(bsize, fp);
            loop {
                let buffer = buf_reader.fill_buf().map_err(|e| e.to_string())?;
                let length = buffer.len();
                if length == 0 {
                    debug!(
                        "read & sent all of input file: {}",
                        opened_buffer.canon_path.to_string_lossy()
                    );
                    break;
                }
                total += length;
                buf_writer.write_all(&buffer).map_err(|e| e.to_string())?;
                trace!("  sent {} / {}", length, total);
                buf_reader.consume(length);
            }
            // Signal we are done sending this file
            let _n = buf_writer
                .write_fmt(format_args!("\n.\n"))
                .map_err(|e| e.to_string());
            debug!(
                "  read {} (out of {} bytes) from input file.",
                total, opened_buffer.size
            );
            info!("Opened {:?}", opened_buffer.canon_path);
        }
    }

    let mut b = [0u8; 512];
    debug!("Waiting for remote editor to identiy itself...");
    let n = socket.recv(&mut b).map_err(|e| e.to_string())?;
    assert!(n < 512);
    debug!(
        "Connected to remote app: {}",
        String::from_utf8_lossy(&b[0..n]).trim()
    );
    Ok(buffers)
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
                close_buffer(&mut opened_buffers, &mut buffer_reader);
            }
            // save the buffer to a file
            "save" => {
                trace!("--> About to call write_to_disk()");
                myline.clear();
                match write_to_disk(&mut opened_buffers, &mut buffer_reader, bsize) {
                    Ok(n) => total += n,
                    Err(e) => error!("Couldn't save: {}", e.to_string()),
                }
            }
            _ => {
                if myline.trim() == "" {
                    trace!("<-- Recvd empty line from editor");
                    continue;
                } else {
                    warn!("***===*** Unrecognized shit: {:?}", myline.trim());
                    return Err(Error::new(ErrorKind::Other, "unrecognized shit"));
                }
            }
        }
    }
    trace!("Cumulative total bytes saved: {}", total);
    Ok(())
}

fn close_buffer(
    opened_buffers: &mut HashMap<String, OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
) {
    let mut myline = String::with_capacity(128);

    while let Ok(n) = buffer_reader.read_line(&mut myline) {
        if n == 0 || myline.trim() == "" {
            trace!("Finished receiving closing instructions");
            break;
        }
        let command: Vec<&str> = myline.trim().splitn(2, ":").collect::<Vec<&str>>();
        trace!("  close instruction:\t{:?}", command);
        let (_, closed_buffer) = opened_buffers.remove_entry(command[1].trim()).unwrap();
        info!("Closed: {:?}", closed_buffer.canon_path.as_os_str());
        myline.clear();
    }
}

fn write_to_disk(
    opened_buffers: &mut HashMap<String, OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
    buf_size: usize,
) -> Result<usize, std::io::Error> {
    let mut myline = String::with_capacity(128);

    buffer_reader.read_line(&mut myline)?;
    trace!("  save instruction:\t{:?}", myline.trim());
    let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0]
        .trim()
        .to_string();
    trace!("  token: >{}<", token);
    myline.clear();

    let t1 = Instant::now();
    let mut total_written = 0usize;
    let mut no_data_chunks = 0usize;
    {
        // Get the info about which file we are receiving data for.
        let rand_temp_file = &mut opened_buffers.get_mut(&token).unwrap().temp_file;
        rand_temp_file.seek(SeekFrom::Start(0))?;

        let mut buf_writer = BufWriter::with_capacity(1024, rand_temp_file);
        // Remote editor may send multiple "data: SIZE" sections under one "save" command
        loop {
            buffer_reader.read_line(&mut myline)?;
            if myline.trim().is_empty() {
                trace!("<- breaking out of write_to_disk");
                break;
            }
            trace!("  -->  save instruction:\t{:?}", myline.trim());
            assert!(myline.trim().contains("data: "));
            no_data_chunks += 1;
            let data_size = myline.rsplitn(2, ":").collect::<Vec<&str>>()[0]
                .trim()
                .parse::<usize>()
                .unwrap();
            trace!("  save size:\t{:?}", data_size);
            myline.clear();

            let mut total = 0usize;
            let reader_len = buffer_reader.buffer().len();
            trace!("  reader len: {}", reader_len);

            let mut buffer = vec![0u8; cmp::max(buffer_reader.buffer().len(), buf_size)];
            let mut chunk_reader = buffer_reader.take(data_size as u64);
            trace!("  buffer len: {}", buffer.len());
            loop {
                let n = chunk_reader.read(&mut buffer)?;
                if n == 0 {
                    trace!("  n = {}", n);
                    total_written += total;
                    trace!("  total_written = {}", total_written);
                    break;
                }
                buf_writer.write_all(&buffer[..n])?;
                total += n;
                trace!(
                    "   - written so far: {}/{}-byte (chunk: {}) to temp file",
                    total,
                    data_size,
                    n
                );
            }
            // loop {
            //     let buffer = buffer_reader.fill_buf()?;
            //     if buffer.is_empty() {
            //         warn!("HMMMMMMMMM..........MMMMMMMMMMMM");
            //         break;
            //     }
            //     let length = buffer.len();
            //     if total + length >= data_size {
            //         trace!("Total recvd: {}", total + length);
            //         trace!("length: {}", length);
            //         let corrected_last_length = data_size - total;
            //         trace!("  data_size: {}", data_size);
            //         trace!("  left over size: {}", length - corrected_last_length);
            //         buf_writer.write_all(&buffer[..corrected_last_length])?;
            //         // trace!( "extra bytes read {}", String::from_utf8_lossy(&buffer[corrected_last_length..]));
            //         buffer_reader.consume(corrected_last_length);
            //         trace!(" -- wrote last chunk: {}", corrected_last_length);
            //         buf_writer.flush()?;
            //         total_written += corrected_last_length;
            //         break;
            //     } else {
            //         buf_writer.write_all(&buffer)?;
            //         total_written += length;
            //         total += length;
            //         trace!(
            //             " -- written so far: {}/{}-byte (chunk: {}) to temp file",
            //             total,
            //             data_size,
            //             length
            //         );
            //         buffer_reader.consume(length);
            //     }
            // }
        }
    }
    let t2 = Instant::now();
    let elapsed = t2 - t1;
    debug!(
        " * time spent saving to temp file: {} micros ({} chunks of save)",
        elapsed.as_micros(),
        no_data_chunks
    );

    debug!("Bytes written to temp file: {}", total_written);
    // Open the file we are supposed to actuallly save to, and copy
    // content of temp. file to it. ensure we only write number of bytes that
    // Sublime Text has sent us.
    {
        // Move file cursor of temp. file to beginning.
        let rand_temp_file = &mut opened_buffers.get_mut(&token).unwrap().temp_file;
        rand_temp_file.seek(SeekFrom::Start(0))?;
    }

    if !opened_buffers.get(&token).unwrap().canwrite {
        info!("File is read-only, not touching it!");
        return Ok(0);
    }

    debug!(
        "About to copy the temp file over the main file ({:?})",
        opened_buffers.get(&token).unwrap().display_name
    );
    opened_buffers
        .get_mut(&token)
        .ok_or(Error::new(
            ErrorKind::Other,
            "can't find the open buffer for saving",
        ))
        .and_then(|opened_buffer| {
            // First we try to create a file name to be used as back up of original one
            let fn_canon = opened_buffer.canon_path.as_path();
            let mut backup_fn_canon = opened_buffer.canon_path.clone();
            let mut backup_fn = backup_fn_canon.file_name().unwrap().to_os_string();

            backup_fn.push("~");
            backup_fn_canon.set_file_name(&backup_fn);

            let mut can_backup = true;
            let mut no_backup_tries = 0;
            while backup_fn_canon.is_file() {
                if no_backup_tries < settings::NO_TRIES_CREATE_BACKUP_FN {
                    no_backup_tries += 1;
                    backup_fn.push("~");
                    backup_fn_canon.set_file_name(&backup_fn);
                    continue;
                } else {
                    warn!(
                        "Cannot backup, Why there is a file named: {}",
                        backup_fn_canon.display()
                    );
                    can_backup = false;
                    break;
                }
            }

            let mut backup = None;
            if can_backup {
                trace!("Backing up to: {}", backup_fn_canon.display());
                if let Err(e) = fs::copy(fn_canon, backup_fn_canon.as_path()) {
                    warn!(
                        "Couldn't write to backup: {} ({})",
                        backup_fn_canon.display(),
                        e.to_string()
                    );
                } else {
                    backup = Some(backup_fn_canon);
                }
            }
            Ok((opened_buffer, backup))
        })
        .and_then(|(opened_buffer, backup)| {
            // Back up the original file before writing over it from temp. file
            let fn_canon = opened_buffer.canon_path.as_path();
            let fp = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(fn_canon)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Other,
                        format!("{}: {:?}", fn_canon.to_string_lossy(), e.to_string()),
                    )
                })?;
            let mut temp_file = File::try_clone(&opened_buffer.temp_file)?;
            temp_file.seek(SeekFrom::Start(0))?;
            let temp_reader_sized = temp_file.take(total_written as u64);

            let mut buffer_writer = BufWriter::new(fp);
            let mut buffer_reader = BufReader::new(temp_reader_sized);

            // Copy from temp over main file (exactly total_written bytes)
            let copy_result = io::copy(&mut buffer_reader, &mut buffer_writer).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("{}: {:?}", fn_canon.to_string_lossy(), e),
                )
            });
            if copy_result.is_ok() {
                Ok((copy_result.unwrap(), fn_canon, backup))
            } else {
                error!("Couldn't save to main file ({})", fn_canon.display());
                error!("  Remote changes are not applied.");
                // If saving didn't succeed, we try to restore from backup
                if let Some(backup_fn) = backup {
                    match fs::copy(&backup_fn, &fn_canon) {
                        // Restored from backup, but changes sent from remote were not applied
                        // Inform the user
                        Ok(_) => trace!(
                            "  Your original file is untouched at: {}",
                            fn_canon.display()
                        ),
                        Err(_e) => {
                            error!("  File on disk may be corrupt but");
                            error!("  its backup is safe at: {}", backup_fn.display());
                            panic!("Halting all operations due to unrceoverable write errors");
                        }
                    }
                    Err(copy_result.unwrap_err())
                }
                // We don't have any backups!!!
                else {
                    error!(
                        "{:?} MAY have been CORRUPTED.",
                        fn_canon.file_name().unwrap()
                    );
                    panic!("Halting all operations due to unrceoverable write errors");
                }
            }
        })
        .and_then(|(written_size, fn_canon, backup)| {
            // Verify # of bytes written & delete backup file
            assert_eq!(total_written as u64, written_size);
            info!("Saved to {:?}", fn_canon);
            if let Some(backup_fn) = backup {
                if let Err(e) = fs::remove_file(&backup_fn) {
                    debug!(
                        "Couldn't remove back up file: {} ({})",
                        backup_fn.display(),
                        e.to_string()
                    );
                } else {
                    trace!("Removed backup file: {:}", backup_fn.display());
                }
            }
            Ok(written_size as usize)
        })
}

// Check if file is writable by user
// metadata.permissions.readonly() checks all bits of file,
// regradless of which user is trying to write to it.
// So it seems actually trying to open the file in write mode is
// the only reliable way of checking the write access of current
// user in a cross platform manner
fn is_writable<P: AsRef<Path>>(p: P, md: &fs::Metadata) -> bool {
    !md.permissions().readonly() && OpenOptions::new().write(true).append(true).open(p).is_ok()
}

// Code for abandoned crate 'hostname'
// let host_name = if let Ok(hostname) = hostname::get() {
//     hostname
// } else {
//     std::ffi::OsString::from("rmate_rust_no_HOST_env_variable")
// };
