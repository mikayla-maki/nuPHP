mod request;

use std::{
    borrow::Cow,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use request::{HttpProtocol, Request};
use thiserror::Error;

#[derive(Error, Debug)]
enum ServerError {
    #[error("400 Bad Request")]
    BadRequest,
    #[error("500 Internal Server Error")]
    InternalServerError,
}

fn main() {
    let address = "127.0.0.1:7878";
    let listener = TcpListener::bind(address).unwrap();
    println!("Test site available at: http://{}", address);
    for stream in listener.incoming() {
        let mut stream = stream.expect("TCP machine broke");

        let response = handle_connection(&mut stream);
        send_response(response, &mut stream);
    }
}
fn send_response(response: Result<String, ServerError>, stream: &mut TcpStream) {
    let (status, body) = match response {
        Ok(response) => (Cow::Borrowed("HTTP/1.1 200 OK"), response),
        Err(error) => (
            Cow::Owned(format!("HTTP/1.1 {}", error)),
            format!("{}", error),
        ),
    };

    // TODO: Something better than this
    let headers = "Content-Type: text/html; charset=utf-8";

    let response = format!("{status}\r\n{headers}\r\n\r\n{body:?}");

    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_connection(stream: &mut TcpStream) -> Result<String, ServerError> {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines().filter_map(|result| result.ok());

    let request_line = lines.next().ok_or(ServerError::BadRequest)?;
    let request: Request = request_line.as_str().try_into()?;
    if !matches!(request.protocol(), HttpProtocol::Http1_1) {
        return Err(ServerError::BadRequest);
    }

    let headers_buffer: Vec<_> = lines.take_while(|line| !line.is_empty()).collect();
    let headers = headers_buffer
        .iter()
        .filter_map(|line| line.split_once(": "))
        .collect::<Vec<_>>();

    dispatch_request(request, headers)
}

fn dispatch_request(request: Request, headers: Vec<(&str, &str)>) -> Result<String, ServerError> {
    todo!();
}
