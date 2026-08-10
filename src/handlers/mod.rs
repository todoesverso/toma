use anyhow::Result;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};

use hyper::body::Incoming as IncomingBody;
use hyper::service::Service;

use std::future::Future;
use std::pin::Pin;

mod utils;

use crate::handlers::utils::{full, not_found_response};

#[derive(Debug, Clone)]
pub struct RequestHandler {}
impl RequestHandler {
    pub fn new() -> Self {
        Self {}
    }

    async fn handle(
        &self,
        req: Request<IncomingBody>,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let path = req.uri().path();
        let _method = req.method().clone();

        match path {
            "/" => full("Home"),
            "/hello" => hello(),
            "/bye" => bye(),
            _ => not_found_response(),
        }
    }
}

impl Service<Request<IncomingBody>> for RequestHandler {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.handle(req).await })
    }
}

fn hello() -> Result<Response<Full<Bytes>>, hyper::Error> {
    full("Hello, World!")
}

fn bye() -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from("Bye, World!"))))
}
