use std::{collections::HashMap, ops::Deref, path::Path};

use hyper::{body::HttpBody, Body, Request};
use url::form_urlencoded;

use crate::ServerError;

#[derive(Clone, Debug)]
pub struct ServerPath<'a> {
    path: &'a Path,
}

impl Deref for ServerPath<'_> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path
    }
}

impl<'a> TryFrom<&'a str> for ServerPath<'a> {
    type Error = ServerError;

    fn try_from(req_path: &'a str) -> Result<Self, Self::Error> {
        if req_path.contains("..") {
            return Err(ServerError::BadRequest);
        }

        let req_path = match req_path.trim_start_matches("/") {
            "" => "index.nu",
            path => path,
        };

        let path = Path::new(req_path);

        Ok(ServerPath { path: &path })
    }
}

pub fn parse_query_params(query: &str) -> Vec<(&str, &str)> {
    let mut params = Vec::new();
    for query_param in query.split("&") {
        let param = query_param.split_once("=").unwrap_or(("", ""));
        params.push(param);
    }
    params
}

#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<Vec<(String, String)>>,
    pub body: Vec<u8>,
}

pub fn nu_record<'a, 'b>(
    i: impl Iterator<Item = &'a (impl AsRef<str> + 'a, impl AsRef<str> + 'a)> + 'b,
) -> String {
    let mut record = String::from("{");
    for (key, val) in i {
        record.push_str(&format!("\"{}\": \"{}\",", key.as_ref(), val.as_ref()));
    }
    record.push_str("}");
    record
}

pub fn nu_map<'a>(i: impl Iterator<Item = (&'a String, &'a Vec<String>)>) -> String {
    let mut record = String::from("{");
    for (key, val) in i {
        record.push_str(&format!(
            "\"{}\": {},",
            key,
            nu_list(val.iter().map(|val| val.as_ref()))
        ));
    }
    record.push_str("}");
    record
}

pub fn nu_list<'a>(i: impl Iterator<Item = &'a str>) -> String {
    let mut record = String::new();
    let mut i = i.peekable();
    let first = i.next();
    if let Some(first) = first {
        if i.peek().is_none() {
            record.push_str("\"");
            record.push_str(first);
            record.push_str("\"");
        } else {
            record.push_str("[\"");
            record.push_str(first);
            record.push_str("\",");
            for val in i {
                record.push_str("\"");
                record.push_str(val);
                record.push_str("\",");
            }
            record.push_str("]");
        }
    } else {
        record.push_str("\"\"");
    }

    record
}

pub struct NuPhpRequest<'a> {
    pub post_body: Vec<(String, String)>,
    pub query_params: Vec<(&'a str, &'a str)>,
    pub headers: HashMap<String, Vec<String>>,
}

impl<'a> NuPhpRequest<'a> {
    pub async fn from(request: &'a mut Request<Body>) -> Result<NuPhpRequest<'a>, ServerError> {
        let upper = request.body().size_hint().upper().unwrap_or(u64::MAX);
        if upper > 1024 * 64 {
            return Err(ServerError::BadRequest);
        }

        let full_body = hyper::body::to_bytes(request.body_mut())
            .await
            .map_err(|_| ServerError::InternalServerError)?;

        let post_body = form_urlencoded::parse(full_body.as_ref())
            .into_owned()
            .collect::<Vec<(String, String)>>();

        let query_params = request
            .uri()
            .query()
            .map(parse_query_params)
            .unwrap_or(vec![]);

        let mut headers = HashMap::<String, Vec<String>>::new();
        for (key, value) in request.headers().iter() {
            if let Ok(value) = value.to_str() {
                headers
                    .entry(key.to_string())
                    .or_default()
                    .push(value.to_string())
            } else {
                continue;
            }
        }

        Ok(NuPhpRequest {
            post_body,
            query_params,
            headers,
        })
    }
}
