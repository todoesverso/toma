use anyhow::Result;
use hyper::header::{self, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use moka::future::Cache;
use tokio::fs::File;

use crate::handlers::full;

static MIME_CACHE: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("html", "text/html"),
        ("htm", "text/html"),
        ("css", "text/css"),
        ("js", "application/javascript"),
        ("mjs", "application/javascript"),
        ("json", "application/json"),
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("gif", "image/gif"),
        ("svg", "image/svg+xml"),
        ("webp", "image/webp"),
        ("ico", "image/x-icon"),
        ("woff", "font/woff"),
        ("woff2", "font/woff2"),
        ("ttf", "font/ttf"),
        ("otf", "font/otf"),
        ("eot", "application/vnd.ms-fontobject"),
        ("txt", "text/plain"),
        ("xml", "application/xml"),
        ("pdf", "application/pdf"),
        ("zip", "application/zip"),
        ("wasm", "application/wasm"),
    ]
    .into_iter()
    .collect()
});

#[inline]
fn get_mime_type_fast(path: &str) -> &'static str {
    PathBuf::from(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| MIME_CACHE.get(ext).copied())
        .unwrap_or("application/octet-stream")
}

struct FileCache {
    cache: Cache<PathBuf, Bytes>,
    max_file_size: u64,
}

impl FileCache {
    pub fn new(max_cache_size: u64, max_file_size: u64) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(max_cache_size)
                .time_to_live(Duration::from_secs(300))
                .build(),
            max_file_size,
        }
    }

    pub async fn get_or_load(&self, path: PathBuf) -> Result<Option<Bytes>> {
        if let Some(data) = self.cache.get(&path).await {
            return Ok(Some(data));
        }

        let file = File::open(&path).await?;
        let metadata = file.metadata().await?;

        // only cache files smaller than max_file_size
        if metadata.len() > self.max_file_size {
            return Ok(None);
        }
        let data = tokio::fs::read(&path).await?;
        let data = Bytes::from(data);
        self.cache.insert(path, data.clone()).await;
        Ok(Some(data))
    }
}

static FILE_CACHE: LazyLock<FileCache> = LazyLock::new(|| {
    FileCache::new(
        100_000_000, // 100 mb total cache
        1_000_000,   // 1MB individual file size to cache
    )
});

pub async fn serve_file(path: &str) -> Response<Full<Bytes>> {
    let path_buf = PathBuf::from(path);
    let mime = get_mime_type_fast(path);

    match FILE_CACHE.get_or_load(path_buf.clone()).await {
        Ok(Some(cached_data)) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, mime)
            .header(CONTENT_LENGTH, cached_data.len())
            .header(CACHE_CONTROL, "public, max-age=3600")
            .body(Full::new(cached_data.clone()))
            .unwrap(),

        _ => full("file too large").unwrap(),
    }
}

#[derive(Debug, Clone)]
pub struct FileHandler {
    pub path: PathBuf,
}

impl FileHandler {
    pub async fn handle(
        &self,
        _req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let path_str = self.path.to_string_lossy().into_owned();
        Ok(serve_file(path_str.as_str()).await)
    }
}
