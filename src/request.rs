use std::path::Path;

use crate::ServerError;

#[derive(Clone, Debug)]
pub struct Request<'a> {
    protocol: HttpProtocol,
    method: HttpMethod,
    request_path: ServerPath<'a>,
}

impl<'a> Request<'a> {
    pub fn protocol(&self) -> HttpProtocol {
        self.protocol
    }

    pub fn path(&self) -> &Path {
        Path::new(self.request_path.path)
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn query_params(&self) -> &[(&str, &str)] {
        &self.request_path.params
    }
}

impl<'a> TryFrom<&'a str> for Request<'a> {
    type Error = ServerError;

    fn try_from(request_line: &'a str) -> Result<Self, Self::Error> {
        let (method, rest) = request_line
            .split_once(" ")
            .ok_or(ServerError::BadRequest)?;

        let (path, protocol) = rest.split_once(" ").ok_or(ServerError::BadRequest)?;

        Ok(Request {
            protocol: protocol.try_into()?,
            method: method.try_into()?,
            request_path: path.try_into()?,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum HttpProtocol {
    Http0_9,
    Http1_0,
    Http1_1,
    Http2_0,
}

impl TryFrom<&str> for HttpProtocol {
    type Error = ServerError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "HTTP/0.9" => Ok(HttpProtocol::Http0_9),
            "HTTP/1.0" => Ok(HttpProtocol::Http1_0),
            "HTTP/1.1" => Ok(HttpProtocol::Http1_1),
            "HTTP/2.0" => Ok(HttpProtocol::Http2_0),
            _ => Err(ServerError::BadRequest),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerPath<'a> {
    path: &'a str,
    params: Vec<(&'a str, &'a str)>,
}

impl<'a> TryFrom<&'a str> for ServerPath<'a> {
    type Error = ServerError;

    fn try_from(path: &'a str) -> Result<Self, Self::Error> {
        if path.contains("..") {
            return Err(ServerError::BadRequest);
        }

        let (path, params) = path
            .split_once("?")
            .map(|(path, params)| {
                (
                    path,
                    params
                        .split("&")
                        .map(|param| param.split_once("=").unwrap_or((param, "")))
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_else(|| (path, vec![]));

        let path = match path.trim_start_matches("/") {
            "" => "index.nu",
            path => path,
        };

        Ok(ServerPath { path, params })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Patch,
}

impl TryFrom<&str> for HttpMethod {
    type Error = ServerError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "TRACE" => Ok(HttpMethod::Trace),
            "CONNECT" => Ok(HttpMethod::Connect),
            "PATCH" => Ok(HttpMethod::Patch),
            _ => Err(ServerError::BadRequest),
        }
    }
}
