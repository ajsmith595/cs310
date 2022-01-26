use core::time;
use std::{
    convert::Infallible,
    fs::File,
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
    }

    drop(listener);
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    while match networking::receive_message(&mut stream) {
        Ok(message) => {
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
                networking::Message::UploadFile => {
                    println!("Receiving file...");
                    let mut output_file = File::create("output-test-file.txt").unwrap();
                    networking::receive_file(&mut stream, &mut output_file);
                    let msg = networking::receive_message(&mut stream).unwrap();

                    println!("Received file! End message: {:?}", msg);
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
                if error.kind() != ErrorKind::WouldBlock {
                    println!("Error type: {:?}", error.kind());
                    println!("Error description: {}", error.to_string());
                    // println!(
                    //     "Error encountered whilst reading from client: {}; shutting down stream",
                    //     error
                    // );
                    // stream.shutdown(Shutdown::Both).unwrap();
                }
                true
            }
        }
    } {
        thread::sleep(time::Duration::from_millis(10));
    }
}
