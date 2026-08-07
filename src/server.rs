use std::net::SocketAddr;

use anyhow::{Context, Result};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use hyper::body::Incoming as IncomingBody;
use hyper::service::Service;

use std::future::Future;
use std::pin::Pin;

pub struct Server {
    // add config in the future
}

#[derive(Debug, Clone)]
pub struct RequestHandler {}
impl RequestHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Service<Request<IncomingBody>> for RequestHandler {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
            Ok(Response::new(Full::new(Bytes::from(s))))
        }

        let res = match req.uri().path() {
            "/" => mk_response("Home".to_string()),
            "/hello" => hello(),
            "/bye" => bye(),
            _ => not_found(),
        };

        Box::pin(async { res })
    }
}

fn hello() -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

fn bye() -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from("Bye, World!"))))
}

fn not_found() -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("404 Not Found")))
        .unwrap())
}

impl Server {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        let listener = TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to socket {addr}"))?;

        loop {
            let (stream, _) = listener
                .accept()
                .await
                .context("Failed to accept connection")?;

            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                let rh = RequestHandler::new();
                if let Err(err) = http1::Builder::new().serve_connection(io, rh).await {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}
