use std::net::TcpStream;

use cs310_shared::networking;
use serde::de;
use serde_json::Error;
use uuid::Uuid;

pub fn receive_uuid(stream: &mut TcpStream) -> Uuid {
    let temp = networking::receive_data(stream, 16).unwrap();
    let mut uuid_bytes = [0 as u8; 16];
    uuid_bytes.copy_from_slice(&temp);
    Uuid::from_bytes(uuid_bytes)
}

pub fn receive_u64(stream: &mut TcpStream) -> u64 {
    let bytes = networking::receive_data(stream, 8).unwrap();
    let mut buffer = [0 as u8; 8];
    buffer.copy_from_slice(&bytes);
    u64::from_ne_bytes(buffer)
}
pub fn receive_u8(stream: &mut TcpStream) -> u8 {
    let bytes = networking::receive_data(stream, 1).unwrap();
    let mut buffer = [0 as u8; 1];
    buffer.copy_from_slice(&bytes);
    u8::from_ne_bytes(buffer)
}
