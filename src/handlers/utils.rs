use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

use crate::handlers::TomaTTPResponse;

pub fn full<T: Into<Bytes>>(chunk: T) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(chunk.into())))
}

pub fn not_found_response() -> Result<TomaTTPResponse> {
    let body = Full::new(Bytes::from("404 Not Found"))
        .map_err(|e| e.into())
        .boxed();
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body)
        .unwrap())
}

pub fn internal_error(err: String) -> Result<TomaTTPResponse> {
    let body = Full::new(Bytes::from(err)).map_err(|e| e.into()).boxed();
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body)
        .unwrap())
}
