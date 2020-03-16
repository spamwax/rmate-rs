use base64;
use socket2::{Domain, Socket, Type};
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::{canonicalize, metadata};
use std::io::prelude::*;
use std::io::{BufRead, BufReader, BufWriter, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr};
// use std::path::PathBuf;

// TODO: make a backup copy of files being saved? <08-03-20, yourname> //
// TODO: create struct to store opsions for each file opened <08-03-20, yourname> //
// TODO: use clap for argument parsing <08-03-20, yourname> //
// TODO: read config files (/etc/rmate.conf)? <08-03-20, yourname> //
// TODO: warn user about openning read-only files <08-03-20, yourname> //
// TODO: use 'envy' crate to parse RMATE_* env. variables. <15-03-20, yourname> //
// TODO: use 'group' feature of clap/structopt to parse: -m name1 namefile1 file1 file2 -m name2 namefile2 file3 <15-03-20, hamid> //

mod settings;
use settings::OpenedBuffer;
use settings::Settings;
use structopt::StructOpt;

fn main() -> Result<(), String> {
    let settings = Settings::from_args();

    println!("verbose: {}", settings.verbose);

    let socket = connect_to_editor(&settings).map_err(|e| e.to_string())?;
    let buffers = get_opened_buffers(&settings)?;
    let buffers = open_file_in_remote(&socket, buffers)?;
    handle_remote(socket, buffers).map_err(|e| e.to_string())?;
    Ok(())
}

fn connect_to_editor(settings: &Settings) -> Result<socket2::Socket, std::io::Error> {
    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();

    // let addr_srv = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port).into();
    // let addr_srv = "127.0.0.1:52698".parse::<SocketAddr>().unwrap().into();
    let addr_srv = "127.0.0.1".parse::<IpAddr>().unwrap();
    let port = settings.port;
    let addr_srv = SocketAddr::new(addr_srv, port).into();

    println!("About to connect to {:?}", addr_srv);
    socket.connect(&addr_srv).unwrap();
    println!(
        "\n\tmy address: {:?}\n\tremote address {:?}\n",
        socket.local_addr().unwrap(),
        socket.peer_addr().unwrap()
    );
    Ok(socket)
}

fn get_opened_buffers(settings: &Settings) -> Result<HashMap<String, OpenedBuffer>, String> {
    let mut buffers = HashMap::new();
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
        let md = metadata(&filename_canon).map_err(|e| e.to_string())?;
        if md.is_dir() {
            return Err("openning directory not supported".to_string());
        }
        let canwrite = !md.permissions().readonly();
        if !(canwrite || settings.force) {
            return Err(format!(
                "{:?} is readonly, use -f/--force to open it anyway",
                file_name_string
            ));
        }
        let filesize = md.len();
        let rand_temp_file = tempfile::tempfile().map_err(|e| e.to_string())?;
        let mut encoded_fn = String::with_capacity(512);
        base64::encode_config_buf(
            filename_canon.to_string_lossy().as_bytes(),
            base64::STANDARD,
            &mut encoded_fn,
        );
        buffers.insert(
            file_name_string.to_string_lossy().into_owned(),
            OpenedBuffer {
                canon_path: filename_canon,
                display_name: file_name_string.clone(),
                canwrite: canwrite,
                metadata: md,
                temp_file: rand_temp_file,
                size: filesize,
            },
        );
    }
    print!("buffers: {:?}\n", &buffers);
    Ok(buffers)
}
fn open_file_in_remote(
    socket: &socket2::Socket,
    buffers: HashMap<String, OpenedBuffer>,
) -> Result<HashMap<String, OpenedBuffer>, String> {
    let bsize = socket.recv_buffer_size().map_err(|e| e.to_string())?;
    print!("recv buffer: {}\n", bsize);
    let bsize = socket.send_buffer_size().map_err(|e| e.to_string())?;
    print!("send buffer: {}\n", bsize);
    let mut total = 0usize;
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, socket);
        for (token, opened_buffer) in buffers.iter() {
            buf_writer
                .write_fmt(format_args!(
                    concat!(
                        "open\ndisplay-name: {}\n",
                        "real-path: {}\ndata-on-save: yes\nre-activate: yes\n",
                        "token: {}\ndata: {}\n"
                    ),
                    opened_buffer.display_name.to_string_lossy(),
                    opened_buffer.canon_path.to_string_lossy(),
                    token,
                    opened_buffer.size,
                ))
                .map_err(|e| e.to_string())?;
            let fp = File::open(&opened_buffer.canon_path).map_err(|e| e.to_string())?;
            let mut buf_reader = BufReader::with_capacity(bsize, fp);
            loop {
                let buffer = buf_reader.fill_buf().map_err(|e| e.to_string())?;
                let length = buffer.len();
                if length == 0 {
                    println!(
                        "read all of file: {}",
                        opened_buffer.canon_path.to_string_lossy()
                    );
                    break;
                }
                total += length;
                buf_writer.write_all(&buffer).map_err(|e| e.to_string())?;
                println!("sent {} ({})", length, total);
                buf_reader.consume(length);
            }
            let _n = buf_writer
                .write_fmt(format_args!("\n.\n"))
                .map_err(|e| e.to_string());
            println!(
                " read {} bytes from file (file size: {})",
                total, opened_buffer.size
            );
        }
    }

    let mut b = [0u8; 512];
    println!("waiting...");
    let n = socket.recv(&mut b).map_err(|e| e.to_string())?;
    assert!(n < 512);
    println!(
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
    println!("waiting for editor's instructions...");
    let mut myline = String::with_capacity(128);
    let bsize = socket.recv_buffer_size()?;
    println!("fofofo:: {}", bsize);
    let mut buffer_reader = BufReader::with_capacity(bsize, &socket);

    // Wait for commands from remote app
    while buffer_reader.read_line(&mut myline)? != 0 {
        println!(
            "\n\n{{{{{{{{{{}}}}}}}}}}\nmyline >{}<\nmyline.trim >>{}<<",
            myline,
            myline.trim()
        );
        match myline.trim() {
            // close the buffer for a file
            "close" => {
                println!("--> in 'close'");
                myline.clear();
                close_buffer(&mut opened_buffers, &mut buffer_reader);
            }
            // save the buffer to a file
            "save" => {
                println!("--> in 'save'");
                myline.clear();
                total += write_to_disk(&mut opened_buffers, &mut buffer_reader)?;
            }
            _ => {
                if myline.trim() == "" {
                    println!("empty line");
                    continue;
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "unrecognized shit",
                    ));
                }
            }
        }
    }
    println!("total bytes: {}", total);
    Ok(())
}

fn close_buffer(
    opened_buffers: &mut HashMap<String, OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
) {
    let mut myline = String::with_capacity(128);

    while let Ok(n) = buffer_reader.read_line(&mut myline) {
        if n == 0 || myline.trim() == "" {
            println!("breaking out of close");
            break;
        }
        let command: Vec<&str> = myline.trim().splitn(2, ":").collect::<Vec<&str>>();
        println!("command:\t{:?}", command);
        // println!("recv token:\t{:?}", command[1].trim());
        let (_, closed_buffer) = opened_buffers.remove_entry(command[1].trim()).unwrap();
        print!("Closed: {:?}\n", closed_buffer.canon_path.as_os_str());
        myline.clear();
    }
}

fn write_to_disk(
    opened_buffers: &mut HashMap<String, OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
) -> Result<usize, std::io::Error> {
    let mut myline = String::with_capacity(128);
    buffer_reader.read_line(&mut myline)?;
    let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0]
        .trim()
        .to_string();
    myline.clear();
    println!("token: >{}<", token);

    buffer_reader.read_line(&mut myline)?;
    let data_size = myline.rsplitn(2, ":").collect::<Vec<&str>>()[0]
        .trim()
        .parse::<usize>()
        .unwrap();
    println!("size: {}", data_size);
    myline.clear();
    println!(
        "token: {:?}\ndisplay-name: {:?}",
        token,
        opened_buffers.get(&token).unwrap().display_name
    );
    let mut total = 0usize;
    {
        let rand_temp_file = &mut opened_buffers.get_mut(&token).unwrap().temp_file;
        rand_temp_file.seek(SeekFrom::Start(0))?;
        let mut buf_writer = BufWriter::with_capacity(1024, rand_temp_file);
        loop {
            let buffer = buffer_reader.fill_buf()?;
            let length = buffer.len();
            total += length;
            if total >= data_size {
                let corrected_last_length = length - (total - data_size);
                println!(
                    "total read: {}, expected size: {}, diff: {}",
                    total,
                    data_size,
                    total - data_size
                );
                buf_writer.write_all(&buffer[..corrected_last_length])?;
                buffer_reader.consume(corrected_last_length);
                buf_writer.flush()?;
                break;
            } else {
                buf_writer.write_all(&buffer)?;
                buffer_reader.consume(length);
            }
        }
    }

    // Open the file we are supposed to actuallly save to, and copy
    // content of temp. file to it. ensure we only write number of bytes that
    // Sublime Text has sent us.
    {
        // Move file cursor of temp. file to beginning.
        let rand_temp_file = &mut opened_buffers.get_mut(&token).unwrap().temp_file;
        rand_temp_file.seek(SeekFrom::Start(0))?;
    }

    opened_buffers
        .get_mut(&token)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "can't find the open buffer for saving",
        ))
        .and_then(|opened_buffer| {
            let fn_canon = opened_buffer.canon_path.as_path();
            let fp = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(fn_canon)?;
            let mut temp_file = File::try_clone(&opened_buffer.temp_file)?;
            temp_file.seek(SeekFrom::Start(0))?;
            let temp_reader_sized = temp_file.take(data_size as u64);

            let mut buffer_writer = BufWriter::new(fp);
            let mut buffer_reader = BufReader::new(temp_reader_sized);
            let written_size = std::io::copy(&mut buffer_reader, &mut buffer_writer)?;
            Ok((written_size, fn_canon))
        })
        .and_then(|(written_size, fn_canon)| {
            assert_eq!(data_size as u64, written_size);
            println!("Saved to {}", fn_canon.to_string_lossy());
            Ok(written_size as usize)
        })
}
