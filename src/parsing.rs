use std::{borrow::Cow, collections::HashMap, ops::Deref, path::Path, sync::Arc};

use http::HeaderMap;
use hyper::{
    body::{Bytes, HttpBody},
    Body, Request,
};
use url::form_urlencoded;

use crate::ServerError;

#[derive(Clone, Debug)]
pub struct ServerPath {
    path: Arc<Path>,
}

impl Deref for ServerPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path.deref()
    }
}

impl<'a> TryFrom<&'a str> for ServerPath {
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

        Ok(ServerPath {
            path: Arc::from(path),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<Vec<(String, String)>>,
    pub body: Vec<u8>,
}

pub fn nu_map<'a>(i: impl Iterator<Item = (impl AsRef<str>, Vec<impl AsRef<str>>)>) -> String {
    let mut record = String::from("{");
    for (key, val) in i {
        record.push_str(&format!(
            "\"{}\": {},",
            key.as_ref(),
            nu_list(val.iter().map(|val| val.as_ref()))
        ));
    }
    record.push_str("}");
    record
}

pub fn nu_headers<'a>(h: &HeaderMap) -> String {
    let mut record = String::from("{");
    for key in h.keys() {
        record.push_str(&format!(
            "\"{}\": {},",
            key,
            nu_list(h.get_all(key).iter().filter_map(|val| val.to_str().ok()))
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

type BorrowedStrMap<'a> = HashMap<Cow<'a, str>, Vec<Cow<'a, str>>>;

pub struct NuPhpRequest<'a> {
    pub post_body: BorrowedStrMap<'a>,
    pub query_params: BorrowedStrMap<'a>,
    pub headers: &'a HeaderMap,
}

impl<'a> NuPhpRequest<'a> {
    pub fn parse_url_encoded(
        body: &'a Bytes,
        request: &'a mut Request<Body>,
    ) -> Result<NuPhpRequest<'a>, ServerError> {
        let upper = request.body().size_hint().upper().unwrap_or(u64::MAX);
        if upper > 1024 * 64 {
            return Err(ServerError::BadRequest);
        }

        let post_body = multi_map(form_urlencoded::parse(body.as_ref()));

        let query_params = request
            .uri()
            .query()
            .map(|query| form_urlencoded::parse(query.as_bytes()))
            .map(multi_map)
            .unwrap_or_default();

        Ok(NuPhpRequest {
            post_body,
            query_params,
            headers: request.headers(),
        })
    }
}

fn multi_map<'a>(
    m: impl Iterator<Item = (impl Into<Cow<'a, str>> + 'a, impl Into<Cow<'a, str>> + 'a)>,
) -> BorrowedStrMap<'a> {
    let mut map: BorrowedStrMap<'a> = HashMap::new();
    for (key, value) in m {
        map.entry(key.into()).or_default().push(value.into())
    }
    map
}
