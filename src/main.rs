use serde::Deserialize;
use socket2::{Domain, Socket, Type};
use std::fs::remove_file;
use std::net::SocketAddr;
use uuid::Uuid;

use serde_derive::{Deserialize, Serialize};

const SRV_SOCKET_FN: &str = ".giv.sock";

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
    let _ = remove_file(SRV_SOCKET_FN);
    // let socket_srv = Socket::new(Domain::unix(), Type::stream(), None).unwrap();
    let socket_srv = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    // let addr_srv = SockAddr::unix(SRV_SOCKET_FN).unwrap();
    let addr_srv = "127.0.0.1:12345".parse::<SocketAddr>().unwrap().into();

    socket_srv.bind(&addr_srv)?;
    println!(" successfull bind to server socket");

    socket_srv.listen(128).unwrap();

    // loop {
    let (sock, addr) = socket_srv.accept().unwrap();
    println!("connection from ==>\n\tsock {:?}\n\taddr: {:?}", sock, addr);
    let mut de = serde_json::Deserializer::from_reader(&sock);
    let item = Item::deserialize(&mut de).unwrap();
    print!("received item is :{}\n", item.name);
    let resp = ClientQuery {
        session_id: Uuid::new_v4(),
        query_id: 2,
        query: "gotyou".to_string(),
    };
    let buff = serde_json::to_string(&resp).unwrap();
    use std::thread::sleep;
    sleep(std::time::Duration::from_millis(3000));
    let n = sock.send(buff.as_bytes()).unwrap();
    println!("  --> {:?}  ({} bytes)", item, n);
    let item = Item::deserialize(&mut de).unwrap();
    print!("received item is :{}\n", item.name);
    println!("Waiting for last message...");
    let item = Item::deserialize(&mut de).unwrap();
    print!("item is :{}\n", item.name);
    // }

    let mut buf = [0u8; 1024];
    let r = sock.recv_from(&mut buf).unwrap();

    let st = String::from_utf8_lossy(&buf[0..r.0]);
    println!("string is: {:?}", st);
    Ok(())
}
