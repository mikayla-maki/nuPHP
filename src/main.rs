mod request;
mod response;

use std::{
    borrow::Cow,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Command, Stdio},
};

use request::{HttpProtocol, Request};
use response::{NuRecord, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
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
fn send_response(response: Result<Response, ServerError>, stream: &mut TcpStream) {
    let (status, headers, body) = match response {
        Ok(response) => (
            Cow::Borrowed("HTTP/1.1 200 OK"),
            response.headers,
            response.body,
        ),
        Err(error) => (
            Cow::Owned(format!("HTTP/1.1 {}", error)),
            None,
            format!("{} :(", error).into_bytes(),
        ),
    };
    let headers = headers
        .map(|headers| {
            headers
                .iter()
                .map(|(key, value)| format!("{}: {}\r\n", key, value))
                .collect::<String>()
        })
        .unwrap_or_else(|| "Content-Type: text/html; charset=utf-8\r\n".to_string());

    let response = format!("{status}\r\n{headers}\r\n");

    stream
        .write_all(response.as_bytes())
        .expect("TCP machine broke");

    stream
        .write_all(body.as_slice())
        .expect("TCP machine broke");
}

fn handle_connection(stream: &mut TcpStream) -> Result<Response, ServerError> {
    let reader = BufReader::new(stream);

    let mut lines = reader.lines().filter_map(|result| result.ok());

    let request_line = lines.next().ok_or(ServerError::BadRequest)?;
    let request: Request = request_line.as_str().try_into()?;
    if !matches!(request.protocol(), HttpProtocol::Http1_1) {
        return Err(ServerError::BadRequest);
    }

    let header_lines = lines.by_ref().take_while(|line| !line.is_empty());
    let headers_buffer = header_lines.collect::<Vec<_>>();

    let headers = headers_buffer
        .iter()
        .filter_map(|line| line.split_once(": "))
        .collect::<Vec<_>>();

    println!("[{:?}] {:?}", request.method(), request.path());

    // let body = lines.collect::<Vec<_>>();
    // dbg!(body);

    dispatch_request(request, headers)
}

fn dispatch_request(request: Request, headers: Vec<(&str, &str)>) -> Result<Response, ServerError> {
    let path = request.path();
    let extension = path.extension();

    if extension.is_none() || extension.unwrap() == "nu" {
        dispatch_nu_file(request, headers)
    } else {
        Ok({
            Response {
                headers: Some(vec![]),
                body: std::fs::read(Path::new("./site/public/").join(path))
                    .map_err(|_| ServerError::InternalServerError)?,
            }
        })
    }
}

fn dispatch_nu_file(request: Request, headers: Vec<(&str, &str)>) -> Result<Response, ServerError> {
    let path = if request.path().extension().is_none() {
        Path::new("./site/public/").join(request.path().with_extension("nu"))
    } else {
        Path::new("./site/public/").join(request.path())
    };

    Command::new("nu")
        .arg("-c")
        .arg(format!(
            r#"
            export-env {{
                $env.GET = {}
                $env.POST = {}
                $env.HEADERS = {}
            }}
            source {}
            "#,
            NuRecord::of(request.query_params()),
            NuRecord::of(&[]),
            NuRecord::of(&headers),
            path.display()
        ))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| ServerError::InternalServerError)
        .and_then(|output| {
            if output.status.success() {
                Ok(Response {
                    headers: None,
                    body: output.stdout,
                })
            } else {
                Err(ServerError::InternalServerError)
            }
        })
}
