use super::settings;
use log::*;
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::File;
use std::fs::{canonicalize, metadata, Metadata};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;
use std::{fs, io};

pub(crate) fn write_to_disk(
    opened_buffers: &mut HashMap<String, settings::OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
    buf_size: usize,
) -> Result<usize, std::io::Error> {
    let mut myline = String::with_capacity(128);

    buffer_reader.read_line(&mut myline)?;
    trace!("  save instruction:\t{:?}", myline.trim());
    let token = myline.trim().rsplitn(2, ':').collect::<Vec<&str>>()[0]
        .trim()
        .to_string();
    trace!("  token: >{}<", token);
    myline.clear();
    let read_only = !opened_buffers.get(&token).unwrap().canwrite;
    if read_only {
        warn!(
            "File is read-only, won't be able to save anything! ({})",
            opened_buffers
                .get(&token)
                .unwrap()
                .canon_path
                .as_path()
                .to_string_lossy()
        );
    }

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
            trace!("-->  save instruction:\t{:?}", myline.trim());
            assert!(myline.trim().contains("data: "));
            no_data_chunks += 1;
            let data_size = myline.rsplitn(2, ':').collect::<Vec<&str>>()[0]
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
                    total_written += total;
                    trace!("  total_written = {}", total_written);
                    buf_writer.flush()?;
                    break;
                }

                if !read_only {
                    buf_writer.write_all(&buffer[..n])?;
                }
                total += n;
                trace!(
                    "   - transferred so far: {}/{}-byte (chunk: {})",
                    total,
                    data_size,
                    n
                );
            }
        }
    }
    let t2 = Instant::now();
    let elapsed = t2 - t1;
    debug!(
        " * time spent reading/writing data: {} micros ({} chunks of save)",
        elapsed.as_micros(),
        no_data_chunks
    );

    debug!("Bytes transferred: {}", total_written);
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
        .ok_or_else(|| Error::new(ErrorKind::Other, "can't find the open buffer for saving"))
        .map(|opened_buffer| {
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
            (opened_buffer, backup)
        })
        .and_then(|(opened_buffer, backup)| {
            // Back up the original file before writing over it from temp. file
            let fn_canon = opened_buffer.canon_path.as_path();
            let fp = fs::OpenOptions::new()
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
            if let Ok(cr) = copy_result {
                buffer_writer.flush()?;
                Ok((cr, fn_canon, backup))
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
        .map(|(written_size, fn_canon, backup)| {
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
            written_size as usize
        })
}

pub(crate) fn get_requested_buffers(
    settings: &settings::Settings,
) -> Result<HashMap<String, settings::OpenedBuffer>, String> {
    let mut buffers = HashMap::new();

    // Iterate over all files and create bookkeeping info
    for (idx, file) in settings.files.iter().enumerate() {
        let filename_canon = if settings.nocreate {
            canonicalize(file).map_err(|e| e.to_string())?
        } else {
            let r = canonicalize(file);
            match r {
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => {
                        info!("Creating new empty file: {:?}", &file);
                        let _ = File::create(file).map_err(|e| e.to_string())?;
                        canonicalize(file).map_err(|e| e.to_string())?
                    }
                    _ => return Err(e.to_string()),
                },
                Ok(_) => {
                    canonicalize(file).map_err(|e| e.to_string())?
                }
            }
        };

        let file_name_string;
        if settings.names.len() > idx {
            file_name_string = settings.names[idx].clone();
        } else {
            file_name_string = filename_canon
                .file_name()
                .ok_or_else(|| "no valid file name found in input argument".to_string())?
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

        let mut disp_name = std::ffi::OsString::with_capacity(128);
        disp_name.push(gethostname::gethostname());
        debug!("Hostname: {:?}", disp_name);
        disp_name.push(":");
        disp_name.push(file_name_string);

        let filesize = md.len();
        let rand_temp_file = tempfile::tempfile().map_err(|e| e.to_string())?;

        let mut hasher = DefaultHasher::new();
        filename_canon.hash(&mut hasher);
        let hashed_fn = hasher.finish();
        trace!("hashed_fn (token): {:x}", hashed_fn);
        if let Some(v) = buffers.insert(
            hashed_fn.to_string(),
            settings::OpenedBuffer {
                canon_path: filename_canon,
                // display_name: file_name_string.clone(),
                display_name: disp_name,
                line,
                filetype,
                canwrite,
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

// Check if file is writable by user
// metadata.permissions.readonly() checks all bits of file,
// regradless of which user is trying to write to it.
// So it seems actually trying to open the file in write mode is
// the only reliable way of checking the write access of current
// user in a cross platform manner
fn is_writable<P: AsRef<Path>>(p: P, md: &Metadata) -> bool {
    !md.permissions().readonly()
        && fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(p)
            .is_ok()
}
