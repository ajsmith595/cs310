use num_traits::FromPrimitive;
use progress_streams::ProgressReader;
use std::env;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{io::Read, net::TcpStream};
use uuid::Uuid;

pub const SERVER_PORT: u16 = 3001; // The port that the server will run on

enum_from_primitive! {
    #[derive(Debug)]
    pub enum Message {
        GetStore,
        GetVideoPreview,
        GetFileThumbnail, // to be implemented
        UploadFile,
        Response,
        EndFile,
        NewChunk,
        AllChunksGenerated,
        GetFileID,
        CompositedClipLength,
        Checksum,
        ChecksumOk,
        ChecksumError,


        CreateSourceClip,
        CreateCompositedClip,
        CreateNode,
        UpdateNode,
        AddLink,
        DeleteLinks,
        UpdateClip,
        DeleteNode,
        CouldNotGeneratePreview,
        CouldNotGetLength,
        DownloadChunk
    }
}

/**
 * Utility function to connect to the server. If SERVER_HOST env variable is not set, localhost is assumed
 */
pub fn connect_to_server() -> Result<TcpStream, Error> {
    let host = match env::var("SERVER_HOST") {
        Ok(host) => host,
        Err(_) => String::from("127.0.0.1"),
    };
    TcpStream::connect(format!("{}:{}", host, SERVER_PORT))
}

/**
 * Sends data through a TcpStream as if it was a file (sends the length of the data)
 */
pub fn send_as_file(stream: &mut TcpStream, file_data: &[u8]) {
    let length = file_data.len().to_ne_bytes();
    send_data(stream, &length).unwrap(); // send the file length
    send_data(stream, file_data).unwrap();
}

/**
 * Sends a file through a TcpStream; sends the length of the file prior so it can be received on the other end via `receive_file`
 */
pub fn send_file(stream: &mut TcpStream, file: &mut File) {
    let file_length = file.metadata().unwrap().len();
    let bytes = file_length.to_ne_bytes();
    send_data(stream, &bytes).unwrap(); // send the file length
    std::io::copy(file, stream).unwrap();
}
/**
 * Sends a file through a TcpStream, whilst also calling the specified callback with the total progress of the transfer
 */
pub fn send_file_with_progress<F>(
    stream: &mut TcpStream,
    file: &mut File,
    callback: F,
) -> Result<(), std::io::Error>
where
    F: Fn(f64, usize),
{
    let file_length = file.metadata().unwrap().len();

    let total = Arc::new(AtomicUsize::new(0));
    let mut reader = ProgressReader::new(file, |progress| {
        total.fetch_add(progress, Ordering::SeqCst);

        let total = total.load(Ordering::SeqCst);
        let perc = (100 * total) as f64 / file_length as f64;
        (callback)(perc, total);
    });

    let bytes = file_length.to_ne_bytes();
    send_data(stream, &bytes)?; // send the file length
    std::io::copy(&mut reader, stream)?;
    Ok(())
}

/**
 * Receives a data stream sent by the sender as a file (with file length), and pushes it into a u8 vector
 */
pub fn receive_file_as_bytes(stream: &mut TcpStream) -> Vec<u8> {
    let file_length = receive_data(stream, 8).unwrap();
    let mut file_length_bytes = [0 as u8; 8];
    file_length_bytes.copy_from_slice(&file_length[0..8]);
    let file_length = u64::from_ne_bytes(file_length_bytes);
    let mut data = stream.take(file_length);

    if file_length > (usize::MAX as u64) {
        panic!("Cannot handle file length greater than max usize value!");
    }
    let file_length = file_length as usize;

    let mut buffer = vec![0; file_length];
    data.read(&mut buffer).unwrap();

    buffer
}

/**
 * Receives file data and pushes it into an output file
 */
pub fn receive_file(stream: &mut TcpStream, output_file: &mut File) {
    let file_length = receive_data(stream, 8).unwrap();
    let mut file_length_bytes = [0 as u8; 8];
    file_length_bytes.copy_from_slice(&file_length[0..8]);
    let file_length = u64::from_ne_bytes(file_length_bytes);

    let mut data = stream.take(file_length);
    std::io::copy(&mut data, output_file).unwrap();
}

/**
 * Sends the relevant message through the TcpStream
 */
pub fn send_message(stream: &mut TcpStream, message: Message) -> Result<(), Error> {
    let mut base = Vec::new();
    base.push(message as u8);

    send_data(stream, base.as_slice())
}

/**
 * Receives a message being sent to the TcpStream
 */
pub fn receive_message(stream: &mut TcpStream) -> Result<Message, Error> {
    let result = receive_data(stream, 1);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    let buffer = result.unwrap();

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

    Ok(message)
}

/**
 * Sends raw data through a TcpStream
 */
pub fn send_data(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), Error> {
    let result = stream.write(bytes);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    Ok(())
}

/**
 * Receives raw data from a TcpStream
 */
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

/**
 * Utility function for receiving a Uuid through a TcpStream
 */
pub fn receive_uuid(stream: &mut TcpStream) -> Result<Uuid, Error> {
    let temp = receive_data(stream, 16)?;
    let mut uuid_bytes = [0 as u8; 16];
    uuid_bytes.copy_from_slice(&temp);
    Ok(Uuid::from_bytes(uuid_bytes))
}

/**
 * Utility function for receiving a u64 through a TcpStream
 */
pub fn receive_u64(stream: &mut TcpStream) -> Result<u64, Error> {
    let bytes = receive_data(stream, 8).unwrap();
    let mut buffer = [0 as u8; 8];
    buffer.copy_from_slice(&bytes);
    Ok(u64::from_ne_bytes(buffer))
}

/**
 * Utility function for receiving a u32 through a TcpStream
 */
pub fn receive_u32(stream: &mut TcpStream) -> Result<u32, Error> {
    let bytes = receive_data(stream, 4).unwrap();
    let mut buffer = [0 as u8; 4];
    buffer.copy_from_slice(&bytes);
    Ok(u32::from_ne_bytes(buffer))
}

/**
 * Utility function for receiving a u16 through a TcpStream
 */
pub fn receive_u16(stream: &mut TcpStream) -> Result<u16, Error> {
    let bytes = receive_data(stream, 2).unwrap();
    let mut buffer = [0 as u8; 2];
    buffer.copy_from_slice(&bytes);
    Ok(u16::from_ne_bytes(buffer))
}

/**
 * Utility function for receiving a u8 through a TcpStream
 */
pub fn receive_u8(stream: &mut TcpStream) -> Result<u8, Error> {
    let bytes = receive_data(stream, 1).unwrap();
    let mut buffer = [0 as u8; 1];
    buffer.copy_from_slice(&bytes);
    Ok(u8::from_ne_bytes(buffer))
}
