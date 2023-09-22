mod parsing;

use std::{
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
use parsing::{nu_map, nu_record, NuPhpRequest, ServerPath};
use rand::{thread_rng, Rng};
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
    move |mut request: Request<Body>| {
        let session_map = session_map.clone();
        Box::pin(async move {
            let request_path = request.uri().path().to_owned();
            let path: ServerPath = request_path
                .as_str()
                .try_into()
                .map_err(|_| ServerError::BadRequest)?;

            println!("[{}] {:?}", request.method(), path.deref());

            let extension = path.extension();
            let session_data = get_session_data(&request, &session_map);

            if extension.is_none() || extension.unwrap() == "nu" {
                let nu_request = NuPhpRequest::from(&mut request).await?;
                dispatch_nu_file(&path, &nu_request, session_data, &session_map)
            } else {
                if let Ok(file) = File::open(Path::new("./site/public/").join(path.deref())).await {
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let body = Body::wrap_stream(stream);
                    return Ok(Response::new(body));
                }

                Ok(not_found())
            }
        })
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
    request: &NuPhpRequest,
    session_data: Option<(String, u64)>,
    session_map: &DashMap<u64, String>,
) -> Result<Response<Body>, ServerError> {
    let path = if path.extension().is_none() {
        Path::new("./site/public/").join(path.with_extension("nu"))
    } else {
        Path::new("./site/public/").join(path.deref())
    };

    let (session_data, key) = session_data.unzip();

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

                # TODO:
                $env.FILES = {{}}
                $env.COOKIES = {{}}
            }}
            source $PATH

            print "{{{{{{{{{{{{HEADERS}}}}}}}}}}}}"
            for $it in ($env.RES_HEADERS | transpose key value) {{
                print $"($it.key): ($it.value)"
            }}

            print "{{{{{{{{{{{{SESSION}}}}}}}}}}}}"
            print ($env.SESSION | to json -r)
            "#,
            path.display(),
            nu_record(request.query_params.iter()),
            nu_record(request.post_body.iter()),
            nu_map(request.headers.iter()),
            session_data
                .as_deref()
                .map(|data| data.trim())
                .unwrap_or_else(|| "{}"),
        ))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| ServerError::InternalServerError)
        .and_then(|output| {
            let output_stdout =
                String::from_utf8(output.stdout).map_err(|_| ServerError::InternalServerError)?;
            let (body, headers_and_session) = output_stdout
                .split_once("{{{{{{HEADERS}}}}}}")
                .expect("we should have added this in the inline script above");
            let (headers, session) = headers_and_session
                .split_once("{{{{{{SESSION}}}}}}")
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
