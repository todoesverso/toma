use anyhow::Result;
use hyper::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE,
};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use futures_util::StreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::{Request, Response, StatusCode};
use moka::future::Cache;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::handlers::{TomaTTPResponse, internal_error};

#[inline]
fn get_mime_type_fast(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("application/octet-stream");

    // A match statement is compiled into a highly efficient jump table/lookup.
    match ext.to_lowercase().as_str() {
        // Web content & text
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript", // Per HTML spec & RFC 9239
        "json" => "application/json",
        "jsonld" => "application/ld+json",
        "webmanifest" => "application/manifest+json",
        "txt" => "text/plain",
        "xml" => "application/xml",
        "xhtml" => "application/xhtml+xml",
        "csv" => "text/csv",
        "md" => "text/markdown",

        // Images
        "png" => "image/png",
        "apng" => "image/apng",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/vnd.microsoft.icon",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",

        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",

        // Audio
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "weba" => "audio/webm",
        "oga" => "audio/ogg",
        "opus" => "audio/ogg", // Opus in Ogg container
        "aac" => "audio/aac",
        "mid" | "midi" => "audio/midi",

        // Video
        "mp4" => "video/mp4",
        "mpeg" => "video/mpeg",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "avi" => "video/x-msvideo",
        "ts" => "video/mp2t",
        "3gp" => "video/3gpp",
        "3g2" => "video/3gpp2",

        // Documents (PDF, Office, etc.)
        "pdf" => "application/pdf",
        "epub" => "application/epub+zip",
        "rtf" => "application/rtf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "abw" => "application/x-abiword",
        "vsd" => "application/vnd.visio",
        "azw" => "application/vnd.amazon.ebook",

        // Archives & Compression
        "zip" => "application/zip",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "bz" => "application/x-bzip",
        "bz2" => "application/x-bzip2",
        "arc" => "application/x-freearc",
        "jar" => "application/java-archive",

        // Applications, scripts & miscellaneous
        "wasm" => "application/wasm",
        "sh" => "application/x-sh",
        "csh" => "application/x-csh",
        "php" => "application/x-httpd-php",
        "ics" => "text/calendar",
        "cda" => "application/x-cdf",
        "mpkg" => "application/vnd.apple.installer+xml",
        "ogx" => "application/ogg",
        "xul" => "application/vnd.mozilla.xul+xml",

        // Default to octet-stream for unknown files (including ".bin")
        _ => "application/octet-stream",
    }
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

    pub async fn get_or_load(&self, path: &PathBuf) -> Result<Option<Bytes>> {
        if let Some(data) = self.cache.get(path).await {
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
        self.cache.insert(path.clone(), data.clone()).await;
        Ok(Some(data))
    }
}

static FILE_CACHE: LazyLock<FileCache> = LazyLock::new(|| {
    FileCache::new(
        100_000_000, // 100 mb total cache
        1_000_000,   // 1MB individual file size to cache
    )
});

/// Unified error handler that gracefully extracts std::io::Error from anyhow::Error
/// Helper function to cleanly separate 404s from 500s
fn handle_error(err: anyhow::Error) -> TomaTTPResponse {
    // Attempt to downcast the anyhow::Error to a std::io::Error
    if let Some(io_err) = err.downcast_ref::<std::io::Error>()
        && io_err.kind() == std::io::ErrorKind::NotFound
    {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(
                Full::new(Bytes::from("404 Not Found"))
                    .map_err(|e| e.into())
                    .boxed(),
            )
            .unwrap();
    }

    // If it's not a 404, return a 500 Internal Server Error
    internal_error(err.to_string()).unwrap()
}

pub async fn serve_full_file(path: &PathBuf, mime: &'static str) -> TomaTTPResponse {
    match FILE_CACHE.get_or_load(path).await {
        // 1 - Load from memory cached
        Ok(Some(cached_data)) => {
            let data_len = cached_data.len();
            let cached_body = Full::new(cached_data).map_err(|e| e.into()).boxed();
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_LENGTH, data_len)
                .header(CACHE_CONTROL, "public, max-age=3600")
                .body(cached_body)
                .unwrap()
        }
        // 2 - File too large for cache, stream the whole file
        Ok(None) => match File::open(path).await {
            Ok(file) => match file.metadata().await {
                Ok(metadata) => {
                    let reader_stream = ReaderStream::new(file);
                    let frame_stream = reader_stream.map(|result| result.map(Frame::data));
                    let body = StreamBody::new(frame_stream).map_err(|e| e.into()).boxed();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, mime)
                        .header(CONTENT_LENGTH, metadata.len())
                        .header(CACHE_CONTROL, "public, max-age=3600")
                        .header(ACCEPT_RANGES, "bytes")
                        .body(body)
                        .unwrap()
                }
                Err(e) => internal_error(e.to_string()).unwrap(),
            },

            Err(e) => handle_error(e.into()),
        },
        // 3 - IO Error
        Err(e) => handle_error(e),
    }
}

/// Handles serving a specific chunk of a file for video streaming / resuming downloads.
async fn serve_partial_file(
    path: &PathBuf,
    range_str: &str,
    mime: &'static str,
) -> TomaTTPResponse {
    let mut file = match File::open(path).await {
        Ok(f) => f,
        Err(e) => return handle_error(e.into()),
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(e) => return internal_error(e.to_string()).unwrap(),
    };

    let file_size = metadata.len();

    // Parse "bytes=START-END"
    let range_data = &range_str[6..];
    let parts: Vec<&str> = range_data.split('-').collect();

    let start: u64 = parts[0].parse().unwrap_or(0);
    let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
        parts[1].parse().unwrap_or(file_size - 1)
    } else {
        file_size - 1
    };

    // spec compliance if the requested range is out of bounds
    if start >= file_size {
        let body = Full::new(Bytes::from("Range Not Satisfiable"))
            .map_err(|e| e.into())
            .boxed();
        return Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(CONTENT_RANGE, format!("bytes */{}", file_size))
            .body(body)
            .unwrap();
    }

    let chunk_size = (end - start) + 1;

    // seek to the stat position
    if let Err(e) = file.seek(std::io::SeekFrom::Start(start)).await {
        return internal_error(e.to_string()).unwrap();
    }

    // stream only the requested chunks
    let reader_stream = ReaderStream::new(file.take(chunk_size));
    let frame_stream = reader_stream.map(|result| result.map(Frame::data));
    let body = StreamBody::new(frame_stream).map_err(|e| e.into()).boxed();
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, mime)
        .header(CONTENT_LENGTH, chunk_size)
        .header(CACHE_CONTROL, "public, max-age=3600")
        .header(ACCEPT_RANGES, "bytes")
        .header(
            CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, file_size),
        )
        .body(body)
        .unwrap()
}

pub async fn serve_file(req: &Request<Incoming>, path: &str) -> TomaTTPResponse {
    let path_buf = PathBuf::from(path);
    let mime = get_mime_type_fast(path);

    // check if the request a specific byte range
    let range_header = req.headers().get(RANGE).and_then(|h| h.to_str().ok());
    if let Some(range_str) = range_header
        && range_str.starts_with("bytes=")
    {
        return serve_partial_file(&path_buf, range_str, mime).await;
    }

    serve_full_file(&path_buf, mime).await
}

#[derive(Debug, Clone)]
pub struct FileHandler {
    pub path: PathBuf,
}

impl FileHandler {
    pub async fn handle(&self, req: Request<Incoming>) -> Result<TomaTTPResponse> {
        let path_str = self.path.to_string_lossy().into_owned();
        Ok(serve_file(&req, path_str.as_str()).await)
    }
}
