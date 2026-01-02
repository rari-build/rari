use async_compression::tokio::write::{BrotliEncoder, GzipEncoder, ZstdEncoder};
use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use tokio::io::AsyncWriteExt;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn compress_stream<S>(
    input: S,
    encoding: CompressionEncoding,
) -> impl Stream<Item = Result<Bytes, std::io::Error>>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
{
    match encoding {
        CompressionEncoding::Zstd => {
            let pinned_input = Box::pin(input);
            futures::stream::unfold(
                (pinned_input, Some(ZstdCompressor::new()), false),
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
                                    error!("Zstd compression error: {}", e);
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
                                        error!("Zstd finalization error: {}", e);
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
            futures::stream::unfold(
                (pinned_input, Some(BrotliCompressor::new()), false),
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
                                    error!("Brotli compression error: {}", e);
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
                                        error!("Brotli finalization error: {}", e);
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
            futures::stream::unfold(
                (pinned_input, Some(GzipCompressor::new()), false),
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
                                    error!("Gzip compression error: {}", e);
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
                                        error!("Gzip finalization error: {}", e);
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
    fn new() -> Self {
        Self { encoder: ZstdEncoder::new(Vec::new()) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> std::io::Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = std::mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> std::io::Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

struct BrotliCompressor {
    encoder: BrotliEncoder<Vec<u8>>,
}

impl BrotliCompressor {
    fn new() -> Self {
        Self { encoder: BrotliEncoder::new(Vec::new()) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> std::io::Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = std::mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> std::io::Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

struct GzipCompressor {
    encoder: GzipEncoder<Vec<u8>>,
}

impl GzipCompressor {
    fn new() -> Self {
        Self { encoder: GzipEncoder::new(Vec::new()) }
    }

    async fn compress_and_flush(&mut self, data: &[u8]) -> std::io::Result<Bytes> {
        self.encoder.write_all(data).await?;
        self.encoder.flush().await?;

        let inner = self.encoder.get_mut();
        let compressed = std::mem::take(inner);

        Ok(Bytes::from(compressed))
    }

    async fn finish(mut self) -> std::io::Result<Bytes> {
        self.encoder.shutdown().await?;
        let final_bytes = self.encoder.into_inner();
        Ok(Bytes::from(final_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_gzip_compression() {
        let data = vec![Ok(Bytes::from("Hello ")), Ok(Bytes::from("World")), Ok(Bytes::from("!"))];

        let input_stream = stream::iter(data);
        let mut compressed_stream = compress_stream(input_stream, CompressionEncoding::Gzip);

        let mut chunks = Vec::new();
        while let Some(chunk) = compressed_stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert!(!chunks.is_empty());
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

        assert!(!chunks.is_empty());
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

        assert!(!chunks.is_empty());
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
