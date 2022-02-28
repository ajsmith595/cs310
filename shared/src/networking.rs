use num_traits::FromPrimitive;
use progress_streams::ProgressReader;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{io::Read, net::TcpStream};

pub const SERVER_HOST: &str = "172.17.115.26";
pub const SERVER_PORT: u16 = 3001;

enum_from_primitive! {
    #[derive(Debug)]
    pub enum Message {
        GetStore,
        SetStore,
        GetVideoPreview,
        GetFileThumbnail,
        UploadFile,
        Response,
        EndFile,
        NewChunk,
        AllChunksGenerated,
        CreateFile,
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
        CouldNotGeneratePreview
    }
}

pub fn connect_to_server() -> Result<TcpStream, Error> {
    TcpStream::connect(format!("{}:{}", SERVER_HOST, SERVER_PORT))
}

pub fn send_as_file(stream: &mut TcpStream, file_data: &[u8]) {
    println!("Sending file (as bytes) of length: {}", file_data.len());
    let length = file_data.len().to_ne_bytes();

    send_data(stream, &length).unwrap(); // send the file length
    send_data(stream, file_data).unwrap();
}

pub fn send_file(stream: &mut TcpStream, file: &mut File) {
    let file_length = file.metadata().unwrap().len();
    let bytes = file_length.to_ne_bytes();
    send_data(stream, &bytes).unwrap(); // send the file length
    std::io::copy(file, stream).unwrap();
}
pub fn send_file_with_progress<F>(stream: &mut TcpStream, file: &mut File, callback: F)
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
    send_data(stream, &bytes).unwrap(); // send the file length
    std::io::copy(&mut reader, stream).unwrap();
}

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

pub fn receive_file(stream: &mut TcpStream, output_file: &mut File) {
    let file_length = receive_data(stream, 8).unwrap();
    let mut file_length_bytes = [0 as u8; 8];
    file_length_bytes.copy_from_slice(&file_length[0..8]);
    let file_length = u64::from_ne_bytes(file_length_bytes);

    let mut data = stream.take(file_length);
    std::io::copy(&mut data, output_file).unwrap();
}

pub fn send_message(stream: &mut TcpStream, message: Message) -> Result<(), Error> {
    send_message_with_data(stream, message, &[])
}
pub fn send_message_with_data(
    stream: &mut TcpStream,
    message: Message,
    bytes: &[u8],
) -> Result<(), Error> {
    println!("Sending message: {:?}", message);
    let mut base = Vec::new();
    base.push(message as u8);
    base.extend_from_slice(bytes);

    send_data(stream, base.as_slice())
}

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
