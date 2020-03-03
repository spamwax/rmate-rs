use serde::Deserialize;
use socket2::{Domain, Socket, Type};
use std::fs::remove_file;
use std::net::SocketAddr;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientQuery {
    session_id: Uuid,
    query_id: u64,
    query: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum Input {
    KeyboardShortcut(String),
    ActionableItem(Item),
    Plugin(ClientQuery),
}

fn main() -> Result<(), std::io::Error> {
    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    // let addr_srv = SockAddr::unix(SRV_SOCKET_FN).unwrap();
    let addr_srv = "127.0.0.1:52696".parse::<SocketAddr>().unwrap().into();
    println!("About to connect to $RMATE_PORT");
    socket.connect(&addr_srv).unwrap();
    println!(
        "\n\tmy address: {:?}\n\tremote address {:?}\n",
        socket.local_addr().unwrap(),
        socket.peer_addr().unwrap()
    );
    let n = socket.send("open\n".as_bytes()).unwrap();
    let n = socket.send("display-name: FUCK-YOU\n".as_bytes()).unwrap();
    let n = socket
        .send("real-path: /home/hamid/fook.txt\n".as_bytes())
        .unwrap();
    let n = socket.send("data-on-save: yes\n".as_bytes()).unwrap();
    let n = socket.send("re-activate: yes\n".as_bytes()).unwrap();
    let n = socket.send("token: fook.txt\n".as_bytes()).unwrap();
    let n = socket.send("data: 807\n".as_bytes()).unwrap();

    use std::fs::File;
    use std::io::{BufRead, BufReader};
    // let mut buf = String::with_capacity(1024);
    // .map_err(|e| e.to_string())
    let mut total = 0usize;
    let mut buf = String::with_capacity(0x1000);
    {
        let f = File::open("/home/hamid/fook.txt")?;
        let mut buf_reader = BufReader::with_capacity(0x1000, f);
        while let Ok(r) = buf_reader.read_line(&mut buf) {
            if r == 0 {
                println!("read last line");
                break;
            }
            println!("******************************");
            println!("{}\n", buf);
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
    // return Ok(());

    // socket_srv.bind(&addr_srv)?;
    // println!(" successfull bind to server socket");

    // socket_srv.listen(128).unwrap();

    // // loop {
    // let (sock, addr) = socket_srv.accept().unwrap();
    // println!("connection from ==>\n\tsock {:?}\n\taddr: {:?}", sock, addr);
    // let mut de = serde_json::Deserializer::from_reader(&sock);
    // let item = Item::deserialize(&mut de).unwrap();
    // print!("received item is :{}\n", item.name);
    // let resp = ClientQuery {
    //     session_id: Uuid::new_v4(),
    //     query_id: 2,
    //     query: "gotyou".to_string(),
    // };
    // let buff = serde_json::to_string(&resp).unwrap();
    // use std::thread::sleep;
    // sleep(std::time::Duration::from_millis(3000));
    // let n = sock.send(buff.as_bytes()).unwrap();
    // println!("  --> {:?}  ({} bytes)", item, n);
    // let item = Item::deserialize(&mut de).unwrap();
    // print!("received item is :{}\n", item.name);
    // println!("Waiting for last message...");
    // let item = Item::deserialize(&mut de).unwrap();
    // print!("item is :{}\n", item.name);
    // // }

    // let mut buf = [0u8; 1024];
    // let r = sock.recv_from(&mut buf).unwrap();

    // let st = String::from_utf8_lossy(&buf[0..r.0]);
    // println!("string is: {:?}", st);
    // Ok(())
}
