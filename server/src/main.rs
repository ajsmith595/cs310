use core::time;
use std::{
    convert::Infallible,
    io::{ErrorKind, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
};

use cs310_shared::{
    networking::{self, SERVER_PORT},
    store::Store,
};
use state::State;
use std::sync::{Arc, Mutex};

mod state;
const OUTPUT_DIR: &str = "output";

fn main() {
    let store = Store::from_file(String::from("state.json"));

    let store = match store {
        Ok(store) => store,
        Err(_) => Store::new(String::from(OUTPUT_DIR)),
    };

    let state = Arc::new(Mutex::new(State { store }));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();

    loop {
        if listener.set_nonblocking(true).is_ok() {
            break;
        }
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                let state = state.clone();
                thread::spawn(move || {
                    handle_client(stream, state);
                });
            }
            Err(e) => {}
        }
        thread::sleep(time::Duration::from_millis(10));
    }

    drop(listener);
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    let mut data = [0 as u8; 50]; // using 50 byte buffer
    while match networking::receive_message(&mut stream) {
        Ok((message, data)) => {
            println!("Valid message received: {:?}", message);

            match message {
                networking::Message::GetStore => {
                    let store = Store::new(String::from(""));

                    let store_json = serde_json::to_string(&store).unwrap();
                    let bytes = store_json.as_bytes();

                    let length = (bytes.len() as u64).to_ne_bytes();
                    networking::send_message_with_data(
                        &mut stream,
                        networking::Message::Response,
                        &length,
                    )
                    .unwrap();
                    // first send the length of the data itself

                    networking::send_data(&mut stream, bytes).unwrap();
                    // then send the data
                }
                _ => println!("Unknown message"),
            }

            true
        }
        Err(error) => {
            if error.kind() == ErrorKind::UnexpectedEof {
                stream.shutdown(Shutdown::Both).unwrap();
                false
            } else {
                println!("Error type: {:?}", error.kind());
                println!("Error description: {}", error.to_string());
                // println!(
                //     "Error encountered whilst reading from client: {}; shutting down stream",
                //     error
                // );
                // stream.shutdown(Shutdown::Both).unwrap();
                true
            }
        }
    } {
        thread::sleep(time::Duration::from_millis(10));
    }
}
