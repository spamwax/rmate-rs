use socket2::{Domain, Socket, Type};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::fs::{canonicalize, metadata};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 {
        return Err("no input file name".to_string());
    }
    let fname = &args[1];
    let socket = open_file_in_remote(fname)?;
    handle_remote(socket);
    Ok(())
}

fn open_file_in_remote(fname: &OsString) -> Result<socket2::Socket, String> {
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

    socket
        .send("open\n".as_bytes())
        .map_err(|e| e.to_string())?;
    socket
        .send(
            ["display-name: ", &file_name_string, "\n"]
                .concat()
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    socket
        .send(
            ["real-path: ", &filename_canon.to_string_lossy(), "\n"]
                .concat()
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    socket
        .send("data-on-save: yes\n".as_bytes())
        .map_err(|e| e.to_string())?;
    socket
        .send("re-activate: yes\n".as_bytes())
        .map_err(|e| e.to_string())?;
    socket
        .send(["token: ", &file_name_string, "\n"].concat().as_bytes())
        .map_err(|e| e.to_string())?;
    let mut data_size = String::with_capacity(1024usize);
    data_size.push_str("data: ");
    data_size.push_str(&filesize.to_string());
    data_size.push_str("\n");
    socket
        .send(data_size.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut total = 0usize;
    let bsize = socket.recv_buffer_size().unwrap();
    println!("hoho:: {}", bsize);

    {
        let mut buf_writer = BufWriter::with_capacity(bsize, &socket);
        let f = File::open(filename_canon).map_err(|e| e.to_string())?;
        let mut buf_reader = BufReader::with_capacity(bsize, f);
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
        // let mut buf = String::with_capacity(0x1000);
        // while let Ok(r) = buf_reader.read_line(&mut buf) {
        //     if r == 0 {
        //         println!("read last line");
        //         break;
        //     }
        //     total += r;
        //     let n = socket.send(buf.as_bytes()).unwrap();
        //     assert_eq!(buf.as_bytes().len(), n);
        //     buf.clear();
        // }
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
    Ok(socket)
}

fn handle_remote(socket: socket2::Socket) -> Result<(), std::io::Error> {
    let mut total = 0usize;
    println!("waiting 2...");
    let mut myline = String::with_capacity(128);
    let mut buf_reader = BufReader::new(&socket);

    // Wait for commands from remote app
    while buf_reader.read_line(&mut myline)? != 0 {
        println!(
            "}}}}}}}}\nmyline >{}<\nmyline.trim >>{}<<",
            myline,
            myline.trim()
        );
        match myline.trim() {
            "close" => {
                println!("--> in 'close'");
                myline.clear();
                while let Ok(n) = buf_reader.read_line(&mut myline) {
                    if n == 0 {
                        break;
                    }
                    println!("-- {}", myline.trim());
                    myline.clear();
                }
            }
            "save" => {
                println!("--> in 'save'");
                myline.clear();
                buf_reader.read_line(&mut myline)?;
                println!("- {}", myline.trim());
                myline.clear();
                buf_reader.read_line(&mut myline)?;
                println!("- {}", myline.trim());
                let data_size = myline.rsplitn(2, ": ").collect::<Vec<&str>>()[0]
                    .trim()
                    .parse::<usize>()
                    .unwrap();
                myline.clear();
                total = 0;
                while let Ok(n) = buf_reader.read_line(&mut myline) {
                    if n == 0 {
                        break;
                    }
                    total += n;
                    println!("* {:?} ({}, {})", myline, myline.len(), total);
                    myline.clear();
                    if total >= data_size {
                        println!("breaking out of save");
                        break;
                    }
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
