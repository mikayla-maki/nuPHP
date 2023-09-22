mod parsing;

use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::Deref,
    path::Path,
    pin::Pin,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use futures::Future;
use http::HeaderValue;
use parsing::{nu_record, NuPhpRequest, ServerPath};
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

    let session_map = Arc::new(Mutex::new(HashMap::new()));
    let session_id_counter = Arc::new(Mutex::new(0));

    let make_svc = make_service_fn(|_conn| {
        let session_map = session_map.clone();
        let session_id_counter = session_id_counter.clone();
        async {
            // service_fn converts our function into a `Service`
            Ok::<_, ServerError>(service_fn(nu_php(session_map, session_id_counter)))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn nu_php(
    session_map: Arc<Mutex<HashMap<u64, String>>>,
    session_counter: Arc<Mutex<u64>>,
) -> impl Fn(
    Request<Body>,
) -> Pin<Box<dyn Future<Output = Result<Response<Body>, ServerError>> + Send>>
       + Send {
    move |mut request: Request<Body>| {
        let session_map = session_map.clone();
        let session_counter = session_counter.clone();
        Box::pin(async move {
            let request_path = request.uri().path().to_owned();
            let path: ServerPath = request_path
                .as_str()
                .try_into()
                .map_err(|_| ServerError::BadRequest)?;

            println!("[{}] {:?}", request.method(), path.deref());

            let extension = path.extension();
            let session_data = get_session_data(&request, session_map);

            if extension.is_none() || extension.unwrap() == "nu" {
                let nu_request = NuPhpRequest::from(&mut request).await?;
                dispatch_nu_file(&path, &nu_request, session_data, session_counter)
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
    session_map: Arc<Mutex<HashMap<u64, String>>>,
) -> Option<String> {
    let session_data = request
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
            session_map.lock().ok().and_then(|mut map| {
                Some(
                    map.entry(key)
                        .or_insert_with(|| "{}".to_string())
                        .to_owned(),
                )
            })
        });
    session_data
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
    session_data: Option<String>,
    session_counter: Arc<Mutex<u64>>,
) -> Result<Response<Body>, ServerError> {
    let path = if path.extension().is_none() {
        Path::new("./site/public/").join(path.with_extension("nu"))
    } else {
        Path::new("./site/public/").join(path.deref())
    };

    Command::new("nu")
        .arg("-c")
        .arg(format!(
            r#"
            export-env {{
                $env.GET = {}
                $env.POST = {}
                $env.HEADERS = {}
                $env.SESSION = {}
            }}
            source {}
            "#,
            nu_record(request.query_params.iter()),
            nu_record(request.post_body.iter()),
            nu_record(request.headers.iter()),
            session_data.as_deref().unwrap_or_else(|| "{}"),
            path.display()
        ))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| ServerError::InternalServerError)
        .and_then(|output| {
            if output.status.success() {
                let mut response = Response::new(Body::from(output.stdout));
                if session_data.is_none() {
                    let mut cookie = session_counter
                        .lock()
                        .map_err(|_| ServerError::InternalServerError)?;
                    *cookie += 1;

                    response.headers_mut().insert(
                        // GENERATE: Set-Cookie: <cookie-name>=<cookie-value>; HttpOnly;
                        "Set-Cookie",
                        HeaderValue::from_str(&format!("{}={}; HttpOnly;", NU_PHP_COOKIE, cookie))
                            .unwrap(),
                    );
                }

                Ok(response)
            } else {
                Err(ServerError::InternalServerError)
            }
        })
}
