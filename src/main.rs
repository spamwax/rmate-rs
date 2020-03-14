use base64;
use socket2::{Domain, Socket, Type};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::{canonicalize, metadata};
use std::io::prelude::*;
use std::io::{BufRead, BufReader, BufWriter, SeekFrom, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

// TODO: make a backup copy of files being saved? <08-03-20, yourname> //
// TODO: create struct to store opsions for each file opened <08-03-20, yourname> //
// TODO: use clap for argument parsing <08-03-20, yourname> //
// TODO: read config files (/etc/rmate.conf)? <08-03-20, yourname> //
// TODO: warn user about openning read-only files <08-03-20, yourname> //

mod settings;
use settings::OpenedBuffer;
use settings::Settings;

fn main() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().collect();
    let mut s = Settings {
        host: "localhost".to_string(),
        port: env::var("RMATE_PORT")
            .unwrap_or("52698".to_string())
            .parse::<u16>()
            .unwrap(),
        wait: true,
        force: false,
        verbose: 1,
        names: vec![],
        files: vec![],
    };

    if args.len() < 2 {
        return Err("no input file name".to_string());
    }
    let fname = args[1].clone();
    s.files.push(fname);
    let (socket, buffers) = open_file_in_remote(&s)?;
    handle_remote(socket, buffers).map_err(|e| e.to_string())?;
    Ok(())
}

fn open_file_in_remote(
    s: &Settings,
) -> Result<(socket2::Socket, HashMap<String, OpenedBuffer>), String> {
    let mut buffers = HashMap::new();
    for (idx, file) in s.files.iter().enumerate() {
        let filename_canon = canonicalize(file).map_err(|e| e.to_string())?;
        let file_name_string;
        if s.names.len() > idx {
            file_name_string = s.names[idx].clone();
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
        let filesize = md.len();
        let rand_temp_file = tempfile::Builder::new()
            .prefix(
                [
                    ".rmate_tmp___",
                    file_name_string.to_str().unwrap_or("use_utf8_plz"),
                    "___",
                ]
                .concat()
                .as_str(),
            )
            .rand_bytes(16)
            .tempfile()
            .map_err(|e| e.to_string())?;
        let mut token = String::with_capacity(512);
        base64::encode_config_buf(
            filename_canon.to_string_lossy().as_bytes(),
            base64::STANDARD,
            &mut token,
        );
        buffers.insert(
            token,
            OpenedBuffer {
                path: filename_canon,
                name: file_name_string.clone(),
                canwrite: canwrite,
                metadata: md,
                temp_file: rand_temp_file,
                size: filesize,
            },
        );
        print!("buffers: {:?}\n", &buffers);
    }

    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();

    // let addr_srv = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port).into();
    // let addr_srv = "127.0.0.1:52698".parse::<SocketAddr>().unwrap().into();
    let addr_srv = "127.0.0.1".parse::<IpAddr>().unwrap();
    let port = s.port;
    let addr_srv = SocketAddr::new(addr_srv, port).into();

    println!("About to connect to {:?}", addr_srv);
    socket.connect(&addr_srv).unwrap();
    println!(
        "\n\tmy address: {:?}\n\tremote address {:?}\n",
        socket.local_addr().unwrap(),
        socket.peer_addr().unwrap()
    );

    let bsize = socket.recv_buffer_size().map_err(|e| e.to_string())?;
    print!("recv buffer: {}\n", bsize);
    let bsize = socket.send_buffer_size().map_err(|e| e.to_string())?;
    print!("send buffer: {}\n", bsize);
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, &socket);
        for buffer in buffers.iter() {
            buf_writer
                .write_fmt(format_args!(
                    concat!(
                        "open\ndisplay-name: {}\n",
                        "real-path: {}\ndata-on-save: yes\nre-activate: yes\n",
                        "token: {}\ndata: {}\n"
                    ),
                    buffer.1.name.to_string_lossy(),
                    buffer.1.path.to_string_lossy(),
                    buffer.0.to_string(),
                    buffer.1.size,
                    // &file_name_string,
                    // &filename_canon.to_string_lossy(),
                    // &file_name_string,
                    // &filesize.to_string()
                ))
                .map_err(|e| e.to_string())?;
        }
    }

    let mut total = 0usize;
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, &socket);
        for opened_buffer in buffers.iter() {
            let fp = File::open(&opened_buffer.1.path).map_err(|e| e.to_string())?;
            let mut buf_reader = BufReader::with_capacity(bsize, fp);
            loop {
                let buffer = buf_reader.fill_buf().map_err(|e| e.to_string())?;
                let length = buffer.len();
                if length == 0 {
                    println!(
                        "read all of file: {}",
                        opened_buffer.1.path.to_string_lossy()
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
                total, opened_buffer.1.size
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
    Ok((socket, buffers))
}

fn handle_remote(
    socket: socket2::Socket,
    mut opened_buffers: HashMap<String, OpenedBuffer>,
) -> Result<(), std::io::Error> {
    let mut total = 0usize;
    println!("waiting 2...");
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
            "close" => {
                println!("--> in 'close'");
                close_buffer(&mut opened_buffers, &mut buffer_reader);
                // myline.clear();
                // while let Ok(n) = buffer_reader.read_line(&mut myline) {
                //     if n == 0 || myline.trim() == "" {
                //         println!("breaking out of close");
                //         break;
                //     }
                //     let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0].trim();
                //     println!("-- {}", token);
                //     myline.clear();
                // }
            }
            "save" => {
                println!("--> in 'save'");
                myline.clear();
                write_to_disk(&mut opened_buffers, &mut buffer_reader)?;
                // let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0].trim();

                // println!("- >{}<", token);
                // myline.clear();
                // buffer_reader.read_line(&mut myline)?;
                // let data_size = myline.rsplitn(2, ":").collect::<Vec<&str>>()[0]
                //     .trim()
                //     .parse::<usize>()
                //     .unwrap();
                // println!("- {}", data_size);
                // myline.clear();
                // total = 0;

                // let token: &str = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0]
                //     .trim()
                //     .clone();
                // let rand_temp_file = &opened_buffers.get(token).unwrap().tempFile;
                // let random_name = rand_temp_file.path();
                // println!("temp file: {:?}", random_name);
                // let mut buf_writer = BufWriter::with_capacity(bsize, rand_temp_file);
                // loop {
                //     let buffer = buffer_reader.fill_buf()?;
                //     let length = buffer.len();
                //     total += length;
                //     if total >= data_size {
                //         let corrected_last_length = length - (total - data_size);
                //         assert_eq!(1, total - data_size);
                //         buf_writer.write_all(&buffer[..corrected_last_length])?;
                //         buffer_reader.consume(corrected_last_length);
                //         break;
                //     } else {
                //         buf_writer.write_all(&buffer)?;
                //         buffer_reader.consume(length);
                //     }
                // }
                // buf_writer.flush()?;

                // let fn_canon = opened_buffers.get(token).as_ref().unwrap().path.as_path();
                // println!("Saved: {}", fn_canon.to_string_lossy());
                // match std::fs::copy(random_name, fn_canon) {
                //     Err(e) => eprintln!(" Error saving: {}", e.to_string()),
                //     Ok(size) => assert_eq!(data_size as u64, size),
                // }
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
    println!("bytes: {}", total);
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
        let command: Vec<&str> = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>();
        println!("recv cmd: {:?}", command);
        opened_buffers.remove_entry(command[0]);
        myline.clear();
    }
}

fn write_to_disk(
    opened_buffers: &mut HashMap<String, OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
) -> Result<(), std::io::Error> {
    let mut myline = String::with_capacity(128);
    buffer_reader.read_line(&mut myline)?;
    let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0]
        .trim()
        .to_string();
    myline.clear();
    println!("- >{}<", token);

    buffer_reader.read_line(&mut myline)?;
    let data_size = myline.rsplitn(2, ":").collect::<Vec<&str>>()[0]
        .trim()
        .parse::<usize>()
        .unwrap();
    println!("- {}", data_size);
    myline.clear();
    let mut total = 0usize;
    {
        let rand_temp_file = &mut opened_buffers.get_mut(&token).unwrap().temp_file;
        println!("temp file name: {:?}", rand_temp_file.path());
        rand_temp_file.seek(SeekFrom::Start(0))?;
        let mut buf_writer = BufWriter::with_capacity(1024, rand_temp_file);
        loop {
            let buffer = buffer_reader.fill_buf()?;
            let length = buffer.len();
            total += length;
            if total >= data_size {
                let corrected_last_length = length - (total - data_size);
                // assert_eq!(1, total - data_size);
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
    let fn_canon = &opened_buffers
        .get(&token)
        .as_ref()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "can't find the open buffer for saving",
        ))?
        .path;

    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(fn_canon)
        .and_then(|fp| {
            let temp_reader_sized = opened_buffers
                .get(&token)
                .unwrap()
                .temp_file
                .as_file()
                .take(data_size as u64);
            let mut buffer_writer = BufWriter::new(fp);
            let mut buffer_reader = BufReader::new(temp_reader_sized);
            std::io::copy(&mut buffer_reader, &mut buffer_writer)
        })
        .and_then(|written_size| {
            assert_eq!(data_size as u64, written_size);
            println!("Saved to {}", fn_canon.to_string_lossy());
            Ok(())
        })
}
