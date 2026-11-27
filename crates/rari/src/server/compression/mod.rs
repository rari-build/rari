use std::io::Error;

pub mod stream;

use bytes::Bytes;
use futures::{StreamExt, stream::once};
pub use stream::{
    CompressionEncoding, CompressionProfile, compress_stream, compress_stream_with_profile,
};

pub async fn compress_body(
    body: Bytes,
    encoding: CompressionEncoding,
) -> (Bytes, CompressionEncoding) {
    compress_body_with_profile(body, encoding, CompressionProfile::PerRequest).await
}

pub async fn compress_body_with_profile(
    body: Bytes,
    encoding: CompressionEncoding,
    profile: CompressionProfile,
) -> (Bytes, CompressionEncoding) {
    if matches!(encoding, CompressionEncoding::Identity) || body.is_empty() {
        return (body, CompressionEncoding::Identity);
    }

    let body_clone = body.clone();
    let input_stream = once(async move { Ok::<Bytes, Error>(body_clone) });
    let mut compressed_stream = compress_stream_with_profile(input_stream, encoding, profile);

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

pub async fn compress_all_encodings(body: Bytes) -> (Option<Bytes>, Option<Bytes>, Option<Bytes>) {
    let profile = CompressionProfile::Precompressed;
    let ((gz, gz_enc), (zs, zs_enc), (br, br_enc)) = tokio::join!(
        compress_body_with_profile(body.clone(), CompressionEncoding::Gzip, profile),
        compress_body_with_profile(body.clone(), CompressionEncoding::Zstd, profile),
        compress_body_with_profile(body, CompressionEncoding::Brotli, profile),
    );
    (
        matches!(gz_enc, CompressionEncoding::Gzip).then_some(gz),
        matches!(zs_enc, CompressionEncoding::Zstd).then_some(zs),
        matches!(br_enc, CompressionEncoding::Brotli).then_some(br),
    )
}
