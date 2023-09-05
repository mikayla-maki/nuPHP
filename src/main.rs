mod parsing;

use std::{
    net::SocketAddr,
    ops::Deref,
    path::Path,
    process::{Command, Stdio},
};

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

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 7878));

    println!("Test site available at: http://{}", addr);

    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, ServerError>(service_fn(nu_php))
    });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn nu_php(mut request: Request<Body>) -> Result<Response<Body>, ServerError> {
    let request_path = request.uri().path().to_owned();
    let path: ServerPath = request_path
        .as_str()
        .try_into()
        .map_err(|_| ServerError::BadRequest)?;

    println!("[{}] {:?}", request.method(), path.deref());

    let extension = path.extension();

    if extension.is_none() || extension.unwrap() == "nu" {
        let nu_request = NuPhpRequest::from(&mut request).await?;
        dispatch_nu_file(&path, &nu_request)
    } else {
        if let Ok(file) = File::open(Path::new("./site/public/").join(path.deref())).await {
            let stream = FramedRead::new(file, BytesCodec::new());
            let body = Body::wrap_stream(stream);
            return Ok(Response::new(body));
        }

        Ok(not_found())
    }
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
            }}
            source {}
            "#,
            nu_record(request.query_params.iter()),
            nu_record(request.post_body.iter()),
            nu_record(request.headers.iter()),
            path.display()
        ))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| ServerError::InternalServerError)
        .and_then(|output| {
            if output.status.success() {
                Ok(Response::new(Body::from(output.stdout)))
            } else {
                Err(ServerError::InternalServerError)
            }
        })
}
