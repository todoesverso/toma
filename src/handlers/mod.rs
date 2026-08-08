use anyhow::Result;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};

use hyper::body::Incoming as IncomingBody;
use hyper::service::Service;

use std::future::Future;
use std::pin::Pin;

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
