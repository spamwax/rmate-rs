use socket2::{Domain, Socket, Type};
use std::env;
use std::ffi::OsString;
use std::fs::{canonicalize, metadata};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() -> Result<(), std::io::Error> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 {
        return Err(Error::new(ErrorKind::Other, "no input file name"));
    }
    let fname = &args[1];
    let md = metadata(fname)?;
    if md.is_dir() {
        return Err(Error::new(
            ErrorKind::Other,
            "openning directory not supported",
        ));
    }
    let filename_canon = canonicalize(fname)?;
    let file_name = filename_canon.file_name().unwrap();
    let filesize = md.len();
    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    let port = env::var("RMATE_PORT")
        .unwrap_or("52696".to_string())
        .parse::<u16>()
        .unwrap();
    // let addr_srv = SockAddr::unix(SRV_SOCKET_FN).unwrap();
    // let addr_srv: socket2::SockAddr = "127.0.0.1:52696".parse::<SocketAddr>().unwrap().into();
    let addr_srv = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port).into();
    println!("About to connect to $RMATE_PORT");
    socket.connect(&addr_srv).unwrap();
    println!(
        "\n\tmy address: {:?}\n\tremote address {:?}\n",
        socket.local_addr().unwrap(),
        socket.peer_addr().unwrap()
    );
    let n = socket.send("open\n".as_bytes()).unwrap();
    let n = socket
        .send(
            ["display-name: ", &file_name.to_string_lossy(), "\n"]
                .concat()
                .as_bytes(),
        )
        .unwrap();
    let n = socket
        .send(
            ["real-path: ", &filename_canon.to_string_lossy(), "\n"]
                .concat()
                .as_bytes(),
        )
        .unwrap();
    let _n = socket.send("data-on-save: yes\n".as_bytes()).unwrap();
    let n = socket.send("re-activate: yes\n".as_bytes()).unwrap();
    let n = socket
        .send(
            ["token: ", &file_name.to_string_lossy(), "\n"]
                .concat()
                .as_bytes(),
        )
        .unwrap();
    let n = socket
        .send(format!("data: {}\n", filesize).as_bytes())
        .unwrap();

    use std::fs::File;
    use std::io::{BufRead, BufReader};
    // let mut buf = String::with_capacity(1024);
    // .map_err(|e| e.to_string())
    let mut total = 0usize;
    let mut buf = String::with_capacity(0x1000);
    {
        let f = File::open(filename_canon)?;
        let mut buf_reader = BufReader::with_capacity(0x1000, f);
        while let Ok(r) = buf_reader.read_line(&mut buf) {
            if r == 0 {
                println!("read last line");
                break;
            }
            // println!("******************************");
            // println!("{}\n", buf);
            total += r;
            let n = socket.send(buf.as_bytes()).unwrap();
            assert_eq!(buf.as_bytes().len(), n);
            buf.clear();
        }
    }

    let n = socket.send("\n.\n".as_bytes()).unwrap();
    println!(" read {} bytes from file", total);

    loop {}
    Ok(())
}
