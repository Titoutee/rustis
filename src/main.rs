use std::{collections::HashMap, sync::{Arc, Mutex}};
use rustis::{StackCtr, handle_connection};
use tokio::net::{TcpListener};

const DB_SZ: usize = 4_096;
const IDS: usize = 100;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6378").await.unwrap();
    let map = Arc::new(Mutex::new(HashMap::with_capacity(DB_SZ)));
    let mut _client_id = StackCtr::init(IDS);
    let client_id = Arc::new(Mutex::new(_client_id));

    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((stream, _)) => {
                let cloned_db = Arc::clone(&map);
                let cloned_id = Arc::clone(&client_id);
                let id = client_id.lock().expect("unlock failed!").get_new_id();
                
                tokio::spawn(async move { handle_connection(stream, id, cloned_db, cloned_id).await });
            }
            Err(e) => {
                println!("Stream error: {}", e);
            }
        }
    }
}
