use std::io::Error;

pub mod stream;

use bytes::Bytes;
use futures::{StreamExt, stream::once};
pub use stream::{CompressionEncoding, compress_stream};

pub async fn compress_body(
    body: Bytes,
    encoding: CompressionEncoding,
) -> (Bytes, CompressionEncoding) {
    if matches!(encoding, CompressionEncoding::Identity) || body.is_empty() {
        return (body, CompressionEncoding::Identity);
    }

    let body_clone = body.clone();
    let input_stream = once(async move { Ok::<Bytes, Error>(body_clone) });
    let mut compressed_stream = compress_stream(input_stream, encoding);

    let mut compressed = Vec::new();
    while let Some(chunk) = compressed_stream.next().await {
        match chunk {
            Ok(data) => compressed.extend_from_slice(&data),
            Err(_) => return (body, CompressionEncoding::Identity),
        }
    }

    if compressed.len() < body.len() {
        (Bytes::from(compressed), encoding)
    } else {
        (body, CompressionEncoding::Identity)
    }
}
