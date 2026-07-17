pub use rari_core::cache::handler;
pub mod loader;
pub use rari_core::cache::response;
pub mod revalidate;
pub mod warmup;
pub use handler::*;

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used, clippy::clone_on_ref_ptr)]
mod tests {
    use std::{env, sync::Arc, time::Instant};

    use axum::http::HeaderMap;
    use bytes::Bytes;
    use rari_core::cache::{
        handler::{CacheHandler, MemoryCacheHandler, MemoryConfig},
        response::{CacheConfig, CachedResponse, CacheMetadata, ResponseCache},
    };

    use crate::server::og::OgImageCache;

    fn create_test_response(body: &str, ttl: u64) -> CachedResponse {
        CachedResponse {
            body: Bytes::from(body.to_string()),
            headers: HeaderMap::new(),
            metadata: CacheMetadata {
                cached_at: Instant::now(),
                ttl,
                etag: Some("test-etag".to_string()),
                tags: vec!["test-tag".to_string()],
            },
            compressed_zstd: None,
            compressed_br: None,
            compressed_gzip: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_namespace_isolates_response_cache_from_og_and_image_layers() {
        let shared: Arc<dyn CacheHandler> =
            Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
                max_entries: 32,
                default_ttl: 60,
            }));

        let response_cache = ResponseCache::new_with_handler(
            CacheConfig { max_entries: 32, default_ttl: 60, enabled: true },
            shared.clone(),
        );
        let test_dir = env::temp_dir().join("rari-test-cache-namespace");
        let og_cache = OgImageCache::with_handler(shared.clone(), &test_dir);

        response_cache
            .set("/about".to_string(), create_test_response("response-body", 60))
            .await;
        let og_payload = vec![0x52, 0x49, 0x46, 0x46];
        og_cache
            .insert("/about".to_string(), og_payload.clone())
            .await
            .expect("og insert");

        let response_got = response_cache.get("/about").await;
        assert!(
            response_got.is_some(),
            "response cache must not be polluted by og write"
        );
        assert_eq!(response_got.unwrap().body, Bytes::from("response-body"));

        let og_got = og_cache.get("/about").await;
        assert_eq!(
            og_got,
            Some(og_payload.clone()),
            "og cache must not be polluted by response write"
        );

        response_cache.invalidate("/about").await;
        let og_after = og_cache.get("/about").await;
        assert_eq!(
            og_after,
            Some(og_payload),
            "og cache must survive response-cache invalidation under shared handler"
        );
    }
}
