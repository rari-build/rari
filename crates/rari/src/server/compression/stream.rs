use std::{io::Result, mem};

use async_compression::{
    Level,
    tokio::write::{BrotliEncoder, GzipEncoder, ZstdEncoder},
};
use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompressionEncoding {
    Zstd,
    Brotli,
    Gzip,
    Identity,
}

impl CompressionEncoding {
    pub fn from_accept_encoding(accept_encoding: Option<&str>) -> Self {
        let accept = accept_encoding.unwrap_or("");

        if accept.contains("zstd") {
            Self::Zstd
        } else if accept.contains("br") {
            Self::Brotli
        } else if accept.contains("gzip") {
            Self::Gzip
        } else {
            Self::Identity
        }
    }

    pub fn as_header_value(&self) -> Option<&'static str> {
        match self {
            Self::Zstd => Some("zstd"),
            Self::Brotli => Some("br"),
            Self::Gzip => Some("gzip"),
            Self::Identity => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompressionProfile {
    PerRequest,
    Precompressed,
}

const PER_REQUEST_BROTLI_QUALITY: i32 = 4;
const PER_REQUEST_GZIP_LEVEL: i32 = 5;

impl CompressionProfile {
    fn brotli_level(self) -> Level {
        match self {
            Self::PerRequest => Level::Precise(PER_REQUEST_BROTLI_QUALITY),
            Self::Precompressed => Level::Best,
        }
    }

    fn gzip_level(self) -> Level {
        match self {
            Self::PerRequest => Level::Precise(PER_REQUEST_GZIP_LEVEL),
            Self::Precompressed => Level::Default,
        }
    }

    fn zstd_level(self) -> Level {
        match self {
            Self::PerRequest | Self::Precompressed => Level::Default,
        }
    }
}

pub fn compress_stream<S>(
    input: S,
    encoding: CompressionEncoding,
) -> impl Stream<Item = Result<Bytes>>
where
    S: Stream<Item = Result<Bytes>> + Send + 'static,
{
    compress_stream_with_profile(input, encoding, CompressionProfile::PerRequest)
}

#[expect(clippy::too_many_lines)]
pub fn compress_stream_with_profile<S>(
    input: S,
    encoding: CompressionEncoding,
    profile: CompressionProfile,
) -> impl Stream<Item = Result<Bytes>>
where
    S: Stream<Item = Result<Bytes>> + Send + 'static,
{
    match encoding {
        CompressionEncoding::Zstd => {
            let pinned_input = Box::pin(input);
            stream::unfold(
                (pinned_input, Some(ZstdCompressor::new(profile.zstd_level())), false),
                |(mut input, mut compressor, finished)| async move {
                    if finished {
                        return None;
                    }

                    match input.next().await {
                        Some(Ok(chunk)) => {
                            let mut comp = compressor.take()?;
                            match comp.compress_and_flush(&chunk).await {
                                Ok(compressed_chunk) => {
                                    Some((Ok(compressed_chunk), (input, Some(comp), false)))
                                }
                                Err(e) => {
                                    tracing::error!("Zstd compression error: {}", e);
                                    Some((Err(e), (input, None, true)))
                                }
                            }
                        }
                        Some(Err(e)) => Some((Err(e), (input, None, true))),
                        None => {
                            if let Some(comp) = compressor.take() {
                                match comp.finish().await {
                                    Ok(final_chunk) => Some((Ok(final_chunk), (input, None, true))),
                                    Err(e) => {
                                        tracing::error!("Zstd finalization error: {}", e);
                                        Some((Err(e), (input, None, true)))
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    }
                },
            )
            .boxed()
        }
        CompressionEncoding::Brotli => {
            let pinned_input = Box::pin(input);
            stream::unfold(
                (pinned_input, Some(BrotliCompressor::new(profile.brotli_level())), false),
                |(mut input, mut compressor, finished)| async move {
                    if finished {
                        return None;
                    }

                    match input.next().await {
                        Some(Ok(chunk)) => {
                            let mut comp = compressor.take()?;
                            match comp.compress_and_flush(&chunk).await {
                                Ok(compressed_chunk) => {
                                    Some((Ok(compressed_chunk), (input, Some(comp), false)))
                                }
                                Err(e) => {
                                    tracing::error!("Brotli compression error: {}", e);
                                    Some((Err(e), (input, None, true)))
                                }
                            }
                        }
                        Some(Err(e)) => Some((Err(e), (input, None, true))),
                        None => {
                            if let Some(comp) = compressor.take() {
                                match comp.finish().await {
                                    Ok(final_chunk) => Some((Ok(final_chunk), (input, None, true))),
                                    Err(e) => {
                                        tracing::error!("Brotli finalization error: {}", e);
                                        Some((Err(e), (input, None, true)))
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    }
                },
            )
            .boxed()
        }
        CompressionEncoding::Gzip => {
            let pinned_input = Box::pin(input);
            stream::unfold(
                (pinned_input, Some(GzipCompressor::new(profile.gzip_level())), false),
                |(mut input, mut compressor, finished)| async move {
                    if finished {
                        return None;
                    }

                    match input.next().await {
                        Some(Ok(chunk)) => {
                            let mut comp = compressor.take()?;
                            match comp.compress_and_flush(&chunk).await {
                                Ok(compressed_chunk) => {
                                    Some((Ok(compressed_chunk), (input, Some(comp), false)))
                                }
                                Err(e) => {
                                    tracing::error!("Gzip compression error: {}", e);
                                    Some((Err(e), (input, None, true)))
                                }
                            }
                        }
                        Some(Err(e)) => Some((Err(e), (input, None, true))),
                        None => {
                            if let Some(comp) = compressor.take() {
                                match comp.finish().await {
                                    Ok(final_chunk) => Some((Ok(final_chunk), (input, None, true))),
                                    Err(e) => {
                                        tracing::error!("Gzip finalization error: {}", e);
                                        Some((Err(e), (input, None, true)))
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    }
                },
            )
            .boxed()
        }
        CompressionEncoding::Identity => Box::pin(input).boxed(),
    }
}

struct ZstdCompressor {
    encoder: ZstdEncoder<Vec<u8>>,
}

impl ZstdCompressor {
    fn new(level: Level) -> Self {
        Self { encoder: ZstdEncoder::with_quality(Vec::new(), level) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

struct BrotliCompressor {
    encoder: BrotliEncoder<Vec<u8>>,
}

impl BrotliCompressor {
    fn new(level: Level) -> Self {
        Self { encoder: BrotliEncoder::with_quality(Vec::new(), level) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

struct GzipCompressor {
    encoder: GzipEncoder<Vec<u8>>,
}

impl GzipCompressor {
    fn new(level: Level) -> Self {
        Self { encoder: GzipEncoder::with_quality(Vec::new(), level) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use std::{
        fmt::Write,
        time::{Duration, Instant},
    };

    use futures::stream;

    use super::*;
    use crate::server::compression::compress_body;

    #[tokio::test]
    async fn test_gzip_compression() {
        let data = vec![Ok(Bytes::from("Hello ")), Ok(Bytes::from("World")), Ok(Bytes::from("!"))];

        let input_stream = stream::iter(data);
        let mut compressed_stream = compress_stream(input_stream, CompressionEncoding::Gzip);

        let mut chunks = Vec::new();
        while let Some(chunk) = compressed_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert_ne!(chunks, Vec::<Bytes>::new());
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    #[tokio::test]
    async fn test_brotli_compression() {
        let data = vec![Ok(Bytes::from("Hello ")), Ok(Bytes::from("World"))];

        let input_stream = stream::iter(data);
        let mut compressed_stream = compress_stream(input_stream, CompressionEncoding::Brotli);

        let mut chunks = Vec::new();
        while let Some(chunk) = compressed_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert_ne!(chunks, Vec::<Bytes>::new());
    }

    #[tokio::test]
    async fn test_zstd_compression() {
        let data = vec![Ok(Bytes::from("Hello ")), Ok(Bytes::from("World")), Ok(Bytes::from("!"))];

        let input_stream = stream::iter(data);
        let mut compressed_stream = compress_stream(input_stream, CompressionEncoding::Zstd);

        let mut chunks = Vec::new();
        while let Some(chunk) = compressed_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert_ne!(chunks, Vec::<Bytes>::new());
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    #[tokio::test]
    async fn test_identity_passthrough() {
        let input_stream = stream::iter(vec![Ok(Bytes::from("Hello ")), Ok(Bytes::from("World"))]);
        let mut compressed_stream = compress_stream(input_stream, CompressionEncoding::Identity);

        let mut chunks = Vec::new();
        while let Some(chunk) = compressed_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], Bytes::from("Hello "));
        assert_eq!(chunks[1], Bytes::from("World"));
    }

    fn large_html_page() -> Bytes {
        let mut html = String::from("<!DOCTYPE html><html><body><table>");
        let mut i: u64 = 0;
        while html.len() < 400 * 1024 {
            i += 1;
            let _ = write!(
                html,
                "<tr class=\"odd:bg-white even:bg-gray-50\"><td class=\"px-4 py-2 font-mono\">{i}</td>\
                 <td class=\"px-4 py-2\">Customer {}</td><td>user{}@example.com</td>\
                 <td class=\"text-right tabular-nums\">${}.{:02}</td></tr>",
                (i * 7919) % 100_003,
                (i * 104_729) % 1_000_003,
                (i * 2_654_435_761) % 10_000,
                i % 100,
            );
        }
        html.push_str("</table></body></html>");
        Bytes::from(html)
    }

    async fn decompress(encoding: CompressionEncoding, data: &[u8]) -> Vec<u8> {
        use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZstdDecoder};
        use tokio::io::AsyncReadExt;

        let mut out = Vec::new();
        match encoding {
            CompressionEncoding::Brotli => BrotliDecoder::new(data).read_to_end(&mut out).await,
            CompressionEncoding::Gzip => GzipDecoder::new(data).read_to_end(&mut out).await,
            CompressionEncoding::Zstd => ZstdDecoder::new(data).read_to_end(&mut out).await,
            CompressionEncoding::Identity => return data.to_vec(),
        }
        .unwrap();
        out
    }

    const LARGE_PAGE_ENCODINGS: [CompressionEncoding; 3] =
        [CompressionEncoding::Brotli, CompressionEncoding::Gzip, CompressionEncoding::Zstd];

    #[tokio::test]
    async fn per_request_compression_round_trips_a_large_page() {
        let page = large_html_page();

        for encoding in LARGE_PAGE_ENCODINGS {
            let (compressed, used) = compress_body(page.clone(), encoding).await;

            assert_eq!(used, encoding);
            assert!(compressed.len() < page.len() / 4, "{encoding:?} barely compressed");
            assert_eq!(decompress(encoding, &compressed).await, page);
        }
    }

    #[tokio::test]
    #[ignore = "timing-sensitive; run on a quiet machine with --ignored"]
    async fn per_request_compression_of_a_large_page_stays_fast() {
        let budget = if cfg!(debug_assertions) {
            Duration::from_millis(300)
        } else {
            Duration::from_millis(20)
        };
        let page = large_html_page();

        for encoding in LARGE_PAGE_ENCODINGS {
            let start = Instant::now();
            let _ = compress_body(page.clone(), encoding).await;
            let elapsed = start.elapsed();

            assert!(elapsed < budget, "{encoding:?} took {elapsed:?} (budget {budget:?})");
        }
    }

    #[test]
    fn test_encoding_from_accept_header() {
        assert_eq!(
            CompressionEncoding::from_accept_encoding(Some("gzip, deflate, br, zstd")),
            CompressionEncoding::Zstd
        );
        assert_eq!(
            CompressionEncoding::from_accept_encoding(Some("gzip, deflate, br")),
            CompressionEncoding::Brotli
        );
        assert_eq!(
            CompressionEncoding::from_accept_encoding(Some("gzip, deflate")),
            CompressionEncoding::Gzip
        );
        assert_eq!(
            CompressionEncoding::from_accept_encoding(Some("identity")),
            CompressionEncoding::Identity
        );
        assert_eq!(CompressionEncoding::from_accept_encoding(None), CompressionEncoding::Identity);
    }
}
