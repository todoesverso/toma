use anyhow::Result;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response};

use hyper::body::Incoming as IncomingBody;
use hyper::service::Service as HyperService;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

mod file;
mod utils;

use crate::handlers::file::FileHandler;
use crate::handlers::utils::{full, internal_error, not_found_response};
use crate::service::{Service, ServiceType};

#[derive(Debug, Clone)]
pub enum DynamicHandler {
    File(FileHandler),
}

impl DynamicHandler {
    pub fn from_config(config: &Service) -> Self {
        match &config.config {
            ServiceType::File { path } => DynamicHandler::File(FileHandler { path: path.clone() }),
            _ => DynamicHandler::File(FileHandler { path: "".into() }),
        }
    }

    pub async fn handle(
        &self,
        req: Request<IncomingBody>,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        match self {
            DynamicHandler::File(handler) => handler.handle(req).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestHandler {
    routes: Arc<HashMap<String, DynamicHandler>>,
}
impl RequestHandler {
    pub fn new(services: Vec<Service>) -> Self {
        let mut routes = HashMap::new();
        for service in services {
            if service.enabled {
                let handler = DynamicHandler::from_config(&service);
                routes.insert(service.route, handler);
            }
        }
        Self {
            routes: Arc::new(routes),
        }
    }

    async fn handle(
        &self,
        req: Request<IncomingBody>,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let path = req.uri().path();
        let _method = req.method().clone();

        if let Some(handler) = self.routes.get(path) {
            handler.handle(req).await
        } else {
            not_found_response()
        }
    }
}

impl HyperService<Request<IncomingBody>> for RequestHandler {
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
