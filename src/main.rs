use dirs;
use serde::Deserialize;
use socket2::SockAddr;
use socket2::{Domain, Socket, Type};
use std::fs::{create_dir_all, remove_file};
use std::io::Error;
use std::thread;
use std::time::Duration;
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

fn main() {
    let mut socket_fn = dirs::data_dir().unwrap();
    socket_fn.push("giv");
    create_dir_all(socket_fn.as_path()).unwrap();
    socket_fn.push(SRV_SOCKET_FN);
    let _ = remove_file(&socket_fn);
    let socket_srv = Socket::new(Domain::unix(), Type::stream(), None).unwrap();
    let addr_srv = SockAddr::unix(&socket_fn).unwrap();

    match socket_srv.bind(&addr_srv) {
        Ok(()) => println!(" bind success to server socket"),
        Err(e) => {
            print!("error is: {:?}", e);
            return;
        }
    }
    socket_srv.listen(128).unwrap();

    loop {
        let (sock, _) = socket_srv.accept().unwrap();
        println!("connection from {:?}", sock);
        let mut de = serde_json::Deserializer::from_reader(&sock);
        let item = Item::deserialize(&mut de).unwrap();
        print!("items is :{}\n\n", item.name);
        let resp = ClientQuery {
            session_id: Uuid::new_v4(),
            query_id: 2,
            query: "gotyou".to_string(),
        };
        let buff = serde_json::to_string(&resp).unwrap();
        let n = sock.send(buff.as_bytes()).unwrap();
        print!(" sent {} bytes\n", n);
        let item = Item::deserialize(&mut de).unwrap();
        print!("items is :{}\n\n", item.name);
        println!("Waiting for last message...");
        let item = Item::deserialize(&mut de).unwrap();
        print!("items is :{}\n", item.name);
    }
}
