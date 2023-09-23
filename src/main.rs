mod parsing;

use std::{
    collections::HashMap,
    io::Write,
    net::SocketAddr,
    ops::Deref,
    path::Path,
    pin::Pin,
    process::{Command, Stdio},
    sync::Arc,
};

use dashmap::DashMap;
use futures::Future;
use http::{HeaderName, HeaderValue};
use parsing::{nu_headers, nu_map, NuPhpRequest, ServerPath};
use rand::{thread_rng, Rng};
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("400 Bad Request")]
    BadRequest,
    #[error("500 Internal Server Error")]
    InternalServerError,
}

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

const NU_PHP_COOKIE: &'static str = "nu_php_cookie";

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 7878));

    println!("Test site available at: http://{}", addr);

    let session_map = Arc::new(DashMap::new());

    let make_svc = make_service_fn(|_conn| {
        let session_map = session_map.clone();
        async { Ok::<_, ServerError>(service_fn(nu_php(session_map))) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn nu_php(
    session_map: Arc<DashMap<u64, String>>,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Result<Response<Body>, ServerError>> + Send>>
       + Send {
    move |request: Request<Body>| {
        let session_map = session_map.clone();
        Box::pin(async move {
            let path: ServerPath = request
                .uri()
                .path()
                .try_into()
                .map_err(|_| ServerError::BadRequest)?;

            println!("[{}] {:?}", request.method(), path.deref());

            let extension = path.extension();
            let session_data = get_session_data(&request, &session_map);

            if extension.is_none() || extension.unwrap() == "nu" {
                build_and_dispatch_nu_request(&path, request, session_data, &session_map).await
            } else {
                if let Ok(file) =
                    File::open(Path::new("./site/public/").join(path.deref().deref())).await
                {
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let body = Body::wrap_stream(stream);
                    return Ok(Response::new(body));
                }

                Ok(not_found())
            }
        })
    }
}

async fn build_and_dispatch_nu_request(
    path: &ServerPath,
    mut request: Request<Body>,
    session_data: Option<(String, u64)>,
    session_map: &DashMap<u64, String>,
) -> Result<Response<Body>, ServerError> {
    match request.headers().get(http::header::CONTENT_TYPE) {
        Some(value) if value == "application/x-www-form-urlencoded" => {
            let full_body = hyper::body::to_bytes(request.body_mut())
                .await
                .map_err(|_| ServerError::InternalServerError)?;

            let nu_request = NuPhpRequest::parse_url_encoded(&full_body, &mut request)?;

            dispatch_nu_file(path, nu_request, session_data, session_map)
        }
        Some(value) => {
            let value = value.to_str().map_err(|_| ServerError::BadRequest)?;
            if value.starts_with("multipart/form-data") {
                let boundary = value
                    .split("boundary=")
                    .nth(1)
                    .ok_or(ServerError::BadRequest)?
                    .to_string();

                let mut multipart = multer::Multipart::new(request.body_mut(), boundary);

                // TODO: Rexamine these data types, String is probably not the best choice
                let mut post_data = HashMap::<String, Vec<String>>::new();
                let mut files = HashMap::<String, Vec<NamedTempFile>>::new();
                while let Some(mut field) = multipart
                    .next_field()
                    .await
                    .map_err(|_| ServerError::InternalServerError)?
                {
                    let Some(name) = field.name() else {
                            // Don't know what to do with this yet
                            continue;
                        };

                    let name = name.to_string();
                    let file_name = field.file_name().map(|file_name| file_name.to_string());
                    println!("Name: {:?}, File Name: {:?}", name, file_name);

                    if let Some(file_name) = file_name {
                        let mut file =
                            NamedTempFile::new().map_err(|_| ServerError::InternalServerError)?;

                        while let Some(chunk) = field
                            .chunk()
                            .await
                            .map_err(|_| ServerError::InternalServerError)?
                        {
                            file.write_all(&chunk[..])
                                .map_err(|_| ServerError::InternalServerError)?;
                        }

                        files.entry(file_name.clone()).or_default().push(file);
                        post_data.entry(name).or_default().push(file_name)
                    } else {
                        let name = name.to_string();

                        let form_data = field
                            .bytes()
                            .await
                            .map_err(|_| ServerError::InternalServerError)?;

                        let form_data = String::from_utf8(form_data.to_vec())
                            .map_err(|_| ServerError::InternalServerError)?;

                        post_data.entry(name).or_default().push(form_data)
                    }
                }

                drop(multipart);

                let nu_request = NuPhpRequest::parse_mulitpart(&post_data, &files, &request)?;
                dispatch_nu_file(path, nu_request, session_data, session_map)
            } else {
                dispatch_nu_file(path, NuPhpRequest::new(&request), session_data, session_map)
            }
        }
        _ => dispatch_nu_file(path, NuPhpRequest::new(&request), session_data, session_map),
    }
}

fn get_session_data(
    request: &Request<Body>,
    session_map: &DashMap<u64, String>,
) -> Option<(String, u64)> {
    request
        .headers()
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            //using the example headers: "yummy_cookie=choco; tasty_cookie=strawberry"
            let key_values = value.split(";");
            let mut cookie_header = None;
            for key_value in key_values {
                let Some((key, value)) = key_value.trim().split_once("=") else {
                    continue;
                };
                if key == NU_PHP_COOKIE {
                    cookie_header = Some(value)
                }
            }
            cookie_header
        })
        .and_then(|value| value.parse::<u64>().ok())
        .and_then(|key| {
            Some((
                session_map
                    .entry(key)
                    .or_insert_with(|| "{}".to_string())
                    .to_owned(),
                key,
            ))
        })
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404: Not Found :(".into())
        .unwrap()
}

fn dispatch_nu_file(
    path: &ServerPath,
    request: NuPhpRequest,
    session_data: Option<(String, u64)>,
    session_map: &DashMap<u64, String>,
) -> Result<Response<Body>, ServerError> {
    let path = if path.extension().is_none() {
        Path::new("./site/public/").join(path.with_extension("nu"))
    } else {
        Path::new("./site/public/").join(path.deref())
    };

    let (session_data, key) = session_data.unzip();

    let token = thread_rng().gen::<u128>();

    // Good enough for multipart form data, good enough for me.
    const DASHES: &'static str = "---------------------------";
    let header_boundary = format!("{}{}HEADER", DASHES, token);
    let session_boundary = format!("{}{}SESSION", DASHES, token);

    Command::new("nu")
        .arg("-c")
        .arg(format!(
            r#"
            const PATH = "{}"
            export-env {{
                $env.GET = {}
                $env.POST = {}
                $env.REQ_HEADERS = {}
                $env.RES_HEADERS = {{}}
                $env.SESSION = {}
                $env.FILES = {}
            }}
            source $PATH

            print "{}"
            for $it in ($env.RES_HEADERS | transpose key value) {{
                print $"($it.key): ($it.value)"
            }}

            print "{}"
            print ($env.SESSION | to json -r)
            "#,
            path.display(),
            nu_map(request.query_params.into_iter()),
            nu_map(request.post_body.into_iter()),
            nu_headers(request.headers),
            session_data
                .as_deref()
                .map(|data| data.trim())
                .unwrap_or_else(|| "{}"),
            nu_map(request.files.into_iter()),
            header_boundary,
            session_boundary
        ))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| ServerError::InternalServerError)
        .and_then(|output| {
            let output_stdout =
                String::from_utf8(output.stdout).map_err(|_| ServerError::InternalServerError)?;
            let (body, headers_and_session) = output_stdout
                .split_once(&header_boundary)
                .expect("we should have added this in the inline script above");
            let (headers, session) = headers_and_session
                .split_once(&session_boundary)
                .expect("we should have added this in the inline script above");

            if output.status.success() {
                let mut response = Response::new(Body::from(body.to_string()));
                let response_headers = response.headers_mut();
                for header in headers.lines() {
                    if header.is_empty() {
                        continue;
                    }

                    let (header, value) = header
                        .split_once(":")
                        .expect("nuphp.nu should have added this");

                    (|| -> Result<(), ServerError> {
                        response_headers.insert(
                            HeaderName::from_bytes(header.as_bytes())
                                .map_err(|_| ServerError::InternalServerError)?,
                            HeaderValue::from_str(value)
                                .map_err(|_| ServerError::InternalServerError)?,
                        );
                        Ok(())
                    })()
                    .ok();
                }

                let key = if let Some(key) = key {
                    key
                } else {
                    let key = thread_rng().gen::<u64>();

                    response.headers_mut().append(
                        http::header::SET_COOKIE,
                        HeaderValue::from_str(&format!(
                            "{}={}; HttpOnly; SameSite;",
                            NU_PHP_COOKIE, key
                        ))
                        .map_err(|_| ServerError::InternalServerError)?,
                    );

                    key
                };

                session_map.insert(key, session.to_string());

                Ok(response)
            } else {
                Err(ServerError::InternalServerError)
            }
        })
}
