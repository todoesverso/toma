use anyhow::Result;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

pub fn full<T: Into<Bytes>>(chunk: T) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(chunk.into())))
}

pub fn not_found_response() -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("404 Not Found")))
        .unwrap())
}
