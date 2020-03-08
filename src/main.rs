use socket2::{Domain, Socket, Type};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::fs::{canonicalize, metadata};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

// TODO: make a backup copy of files being saved? <08-03-20, yourname> //
// TODO: create struct to store opsions for each file opened <08-03-20, yourname> //
// TODO: use clap for argument parsing <08-03-20, yourname> //
// TODO: read config files (/etc/rmate.conf)? <08-03-20, yourname> //
// TODO: warn user about openning read-only files <08-03-20, yourname> //

fn main() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 {
        return Err("no input file name".to_string());
    }
    let fname = &args[1];
    let (socket, filename_canon) = open_file_in_remote(fname)?;
    handle_remote(socket, filename_canon).map_err(|e| e.to_string())?;
    Ok(())
}

fn open_file_in_remote(fname: &OsString) -> Result<(socket2::Socket, PathBuf), String> {
    let filename_canon = canonicalize(fname).map_err(|e| e.to_string())?;
    let file_name = filename_canon
        .file_name()
        .ok_or("no valid file name found in input argument".to_string())?;
    let file_name_string = file_name.to_string_lossy();

    let md = metadata(fname).map_err(|e| e.to_string())?;
    if md.is_dir() {
        return Err("openning directory not supported".to_string());
    }
    let filesize = md.len();

    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    let port = env::var("RMATE_PORT")
        .unwrap_or("52696".to_string())
        .parse::<u16>()
        .unwrap();

    let addr_srv = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port).into();
    println!("About to connect to {:?}", addr_srv);
    socket.connect(&addr_srv).unwrap();
    println!(
        "\n\tmy address: {:?}\n\tremote address {:?}\n",
        socket.local_addr().unwrap(),
        socket.peer_addr().unwrap()
    );

    let bsize = socket.recv_buffer_size().map_err(|e| e.to_string())?;
    print!("hohoho: {}\n", bsize);
    let bsize = socket.send_buffer_size().map_err(|e| e.to_string())?;
    print!("hohoho: {}\n", bsize);
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, &socket);
        buf_writer
            .write_fmt(format_args!(
                concat!(
                    "open\ndisplay-name: {}\n",
                    "real-path: {}\ndata-on-save: yes\nre-activate: yes\n",
                    "token: {}\ndata: {}\n"
                ),
                &file_name_string,
                &filename_canon.to_string_lossy(),
                &file_name_string,
                &filesize.to_string()
            ))
            .map_err(|e| e.to_string())?;
    }

    let mut total = 0usize;
    {
        let mut buf_writer = BufWriter::with_capacity(bsize, &socket);
        let fp = File::open(&filename_canon).map_err(|e| e.to_string())?;
        let mut buf_reader = BufReader::with_capacity(bsize, fp);
        loop {
            let buffer = buf_reader.fill_buf().map_err(|e| e.to_string())?;
            let length = buffer.len();
            if length == 0 {
                println!("read all of file");
                break;
            }
            total += length;
            println!("sent {} ({})", length, total);
            buf_writer.write_all(&buffer).map_err(|e| e.to_string())?;
            buf_reader.consume(length);
        }
    }

    let _n = socket.send("\n.\n".as_bytes()).map_err(|e| e.to_string())?;
    println!(" read {} bytes from file (file size: {})", total, filesize);
    let mut b = [0u8; 512];
    println!("waiting...");
    let n = socket.recv(&mut b).map_err(|e| e.to_string())?;
    assert!(n < 512);
    println!(
        "Connected to remote app: {}",
        String::from_utf8_lossy(&b[0..n]).trim()
    );
    Ok((socket, filename_canon))
}

fn handle_remote(socket: socket2::Socket, filename_canon: PathBuf) -> Result<(), std::io::Error> {
    let mut total = 0usize;
    println!("waiting 2...");
    let mut myline = String::with_capacity(128);
    let bsize = socket.recv_buffer_size()?;
    println!("fofofo:: {}", bsize);
    let mut buf_reader = BufReader::with_capacity(bsize, &socket);

    // Wait for commands from remote app
    while buf_reader.read_line(&mut myline)? != 0 {
        println!(
            "{{{{{{{{{{}}}}}}}}}}}}\nmyline >{}<\nmyline.trim >>{}<<",
            myline,
            myline.trim()
        );
        match myline.trim() {
            "close" => {
                println!("--> in 'close'");
                myline.clear();
                while let Ok(n) = buf_reader.read_line(&mut myline) {
                    if n == 0 || myline.trim() == "" {
                        println!("breaking out of close");
                        break;
                    }
                    let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0].trim();
                    println!("-- {}", token);
                    myline.clear();
                }
            }
            "save" => {
                println!("--> in 'save'");
                myline.clear();
                buf_reader.read_line(&mut myline)?;
                let token = myline.trim().rsplitn(2, ":").collect::<Vec<&str>>()[0].trim();
                println!("- >{}<", token);
                myline.clear();
                buf_reader.read_line(&mut myline)?;
                let data_size = myline.rsplitn(2, ":").collect::<Vec<&str>>()[0]
                    .trim()
                    .parse::<usize>()
                    .unwrap();
                println!("- {}", data_size);
                myline.clear();
                total = 0;

                let rand_temp_file = tempfile::Builder::new()
                    .prefix(".rmate_tmp_")
                    .rand_bytes(8)
                    .suffix(&"~")
                    .tempfile()?;
                let random_name = rand_temp_file.path();
                println!("temp file: {:?}", random_name);
                let mut buf_writer = BufWriter::with_capacity(bsize, &rand_temp_file);
                loop {
                    let buffer = buf_reader.fill_buf()?;
                    let length = buffer.len();
                    total += length;
                    if total >= data_size {
                        // println!("{}", String::from_utf8_lossy(&buffer.clone()));
                        let corrected_last_lenght = length - (total - data_size);
                        assert_eq!(1, total - data_size);
                        buf_writer.write_all(&buffer[..corrected_last_lenght])?;
                        buf_reader.consume(corrected_last_lenght);
                        // println!("breaking out of save {} / {}", data_size, total);
                        break;
                    } else {
                        buf_writer.write_all(&buffer)?;
                        buf_reader.consume(length);
                    }
                }
                buf_writer.flush()?;

                println!("Saving: {}", filename_canon.to_str().unwrap());
                match std::fs::copy(random_name, &filename_canon) {
                    Err(e) => eprintln!(" Error saving: {}", e.to_string()),
                    Ok(size) => assert_eq!(data_size as u64, size),
                }
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
