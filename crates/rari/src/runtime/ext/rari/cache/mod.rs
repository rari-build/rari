use deno_core::{Extension, ExtensionArguments};

use crate::server::config::CacheLayerConfig;

pub mod redb_cache;
pub mod redis_cache;

fn remote_layer_has_url(layer: &CacheLayerConfig) -> bool {
    layer.url.as_deref().map(str::trim).is_some_and(|url| !url.is_empty())
}

fn configured_remote_handler(layer: &CacheLayerConfig) -> Option<&'static str> {
    match layer.handler.as_str() {
        "test" => Some("test"),
        "redis" if remote_layer_has_url(layer) => Some("redis"),
        "redb" if remote_layer_has_url(layer) => Some("redb"),
        _ => None,
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    extensions.extend(redis_cache::extensions(None, is_snapshot));
    extensions.extend(redb_cache::extensions(None, is_snapshot));
    (extensions, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::{configured_remote_handler, remote_layer_has_url};
    use crate::server::config::CacheLayerConfig;

    fn layer(handler: &str, url: Option<&str>) -> CacheLayerConfig {
        CacheLayerConfig {
            handler: handler.to_string(),
            url: url.map(String::from),
            max_entries: 1000,
            default_ttl_secs: 60,
            ..Default::default()
        }
    }

    #[test]
    fn remote_layer_has_url_rejects_blank_values() {
        assert!(!remote_layer_has_url(&layer("redis", None)));
        assert!(!remote_layer_has_url(&layer("redis", Some(""))));
        assert!(!remote_layer_has_url(&layer("redis", Some("   "))));
        assert!(remote_layer_has_url(&layer("redis", Some("redis://localhost:6379"))));
    }

    #[test]
    fn test_handler_is_configured() {
        assert_eq!(configured_remote_handler(&layer("test", None)), Some("test"));
    }

    #[test]
    fn redis_handler_requires_url() {
        assert_eq!(
            configured_remote_handler(&layer("redis", Some("redis://localhost:6379"))),
            Some("redis")
        );
        assert_eq!(configured_remote_handler(&layer("redis", None)), None);
    }

    #[test]
    fn redb_handler_requires_url() {
        assert_eq!(
            configured_remote_handler(&layer("redb", Some("/tmp/cache.redb"))),
            Some("redb")
        );
        assert_eq!(configured_remote_handler(&layer("redb", None)), None);
    }

    #[test]
    fn unknown_handler_is_not_configured() {
        assert_eq!(configured_remote_handler(&layer("memory", None)), None);
    }
}
