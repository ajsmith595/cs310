use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind, Write};
use std::{io::Read, net::TcpStream};

pub const SERVER_HOST: &str = "127.0.0.1";
pub const SERVER_PORT: u16 = 3000;

enum_from_primitive! {
    #[derive(Debug)]
    pub enum Message {
        GetStore,
        SetStore,
        GetVideoPreview,
        GetFileThumbnail,
        UploadFile,
        Response
    }
}

pub fn send_message(stream: &mut TcpStream, message: Message) -> Result<(), Error> {
    send_message_with_data(stream, message, &[])
}
pub fn send_message_with_data(
    stream: &mut TcpStream,
    message: Message,
    bytes: &[u8],
) -> Result<(), Error> {
    let mut base = Vec::new();
    base.push(message as u8);
    base.extend_from_slice(bytes);

    send_data(stream, base.as_slice())
}

pub fn receive_message(stream: &mut TcpStream) -> Result<(Message, Vec<u8>), Error> {
    let result = receive_data(stream, 256);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    let mut buffer = result.unwrap();

    if buffer.len() == 0 {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            format!("Buffer is empty!"),
        ));
    }

    let message = match Message::from_u8(buffer[0]) {
        Some(message) => message,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Message is invalid"),
            ))
        }
    };

    buffer.remove(0);

    Ok((message, buffer))
}

pub fn send_data(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), Error> {
    let result = stream.write(bytes);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    Ok(())
}
pub fn receive_data(stream: &mut TcpStream, buffer_size: u64) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![0; buffer_size as usize];

    let result = stream.read(&mut buffer);

    if result.is_err() {
        return Err(result.unwrap_err());
    }
    let length = result.unwrap();
    buffer.truncate(length);
    Ok(buffer)
}
