use socket2::{Domain, Socket, Type};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::fs::{canonicalize, metadata};
use std::io::{BufRead, BufReader};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() -> Result<(), std::io::Error> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 {
        return Err(Error::new(ErrorKind::Other, "no input file name"));
    }
    let fname = &args[1];
    let filename_canon = canonicalize(fname)?;
    let file_name = filename_canon.file_name().ok_or(Error::new(
        ErrorKind::Other,
        "no valid file name found in input argument",
    ))?;
    let md = metadata(fname)?;
    if md.is_dir() {
        return Err(Error::new(
            ErrorKind::Other,
            "openning directory not supported",
        ));
    }
    let filesize = md.len();
    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    let port = env::var("RMATE_PORT")
        .unwrap_or("52696".to_string())
        .parse::<u16>()
        .unwrap();

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

    let _n = socket.send("\n.\n".as_bytes()).unwrap();
    println!(" read {} bytes from file", total);

    // socket.listen(128).unwrap();
    let mut b = [0u8; 100];
    println!("waiting...");
    let _r = socket.recv(&mut b);
    println!("got shit: {:?}", String::from_utf8_lossy(&b));
    println!("waiting 2...");
    let mut myline = Vec::with_capacity(8 * 1024usize);
    let mut buf_reader = BufReader::new(&socket);
    total = 0;
    let mut c = 0;
    while let Ok(n) = buf_reader.read_until(b'\n', &mut myline) {
        c += 1;
        if n == 0 {
            break;
        }
        println!(
            "got shit 2: {:?}  ({}, {}, {})",
            String::from_utf8_lossy(myline.as_ref()),
            n,
            c,
            total
        );
        myline.clear();
        if c > 3 {
            total += n;
        }
    }
    println!("bytes: {}", total);

    Ok(())
}
