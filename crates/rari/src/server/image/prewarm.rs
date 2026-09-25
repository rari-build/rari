use std::{
    collections::HashSet,
    sync::{Arc, LazyLock, Mutex, PoisonError},
};

use regex::Regex;
use url::form_urlencoded;

use super::{
    optimizer::ImageOptimizer,
    types::{DEFAULT_IMAGE_QUALITY, OptimizeParams},
};
use crate::server::ServerState;

static IMAGE_OPTIMIZE_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    Regex::new(r#"/_rari/image\?([^"'\\\s>]+)"#).expect("image optimize URL regex must compile")
});

static PREWARM_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

struct PrewarmInFlightGuard {
    key: String,
}

impl Drop for PrewarmInFlightGuard {
    fn drop(&mut self) {
        let mut in_flight = PREWARM_IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner);
        in_flight.remove(&self.key);
    }
}

fn try_claim_prewarm_key(key: String) -> Option<PrewarmInFlightGuard> {
    let mut in_flight = PREWARM_IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner);
    if !in_flight.insert(key.clone()) {
        return None;
    }
    Some(PrewarmInFlightGuard { key })
}

pub fn extract_optimize_params_from_html(html: &str) -> Vec<OptimizeParams> {
    let mut seen = HashSet::new();
    let mut params = Vec::new();

    for caps in IMAGE_OPTIMIZE_URL_REGEX.captures_iter(html) {
        let Some(raw_query) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let query = raw_query.replace("&amp;", "&");
        let Some(param) = parse_optimize_query(&query) else {
            continue;
        };
        let key = optimize_dedupe_key(&param);
        if seen.insert(key) {
            params.push(param);
        }
    }

    params
}

fn unique_source_urls(params: &[OptimizeParams]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut urls = Vec::new();
    for param in params {
        if seen.insert(param.url.clone()) {
            urls.push(param.url.clone());
        }
    }
    urls
}

fn parse_optimize_query(query: &str) -> Option<OptimizeParams> {
    let mut url = None;
    let mut w = None;
    let mut q = DEFAULT_IMAGE_QUALITY;
    let mut f = None;

    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        match key.as_ref() {
            "url" if !value.is_empty() => url = Some(value.into_owned()),
            "w" => w = value.parse().ok(),
            "q" => {
                if let Ok(parsed) = value.parse() {
                    q = parsed;
                }
            }
            "f" if !value.is_empty() => f = Some(value.into_owned()),
            _ => {}
        }
    }

    Some(OptimizeParams { url: url?, w, q, f, blur: None })
}

fn optimize_dedupe_key(params: &OptimizeParams) -> String {
    format!(
        "{}|{}|{}|{}",
        params.url,
        params.w.unwrap_or(0),
        params.q,
        params.f.as_deref().unwrap_or("avif")
    )
}

fn blur_dedupe_key(url: &str) -> String {
    format!("blur|{url}")
}

pub fn schedule_prewarm_from_html(optimizer: Arc<ImageOptimizer>, html: &str) {
    let params = extract_optimize_params_from_html(html);
    if params.is_empty() {
        return;
    }
    let blur_urls = unique_source_urls(&params);

    tokio::spawn(async move {
        let mut warmed = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        for param in params {
            let key = optimize_dedupe_key(&param);
            let Some(_guard) = try_claim_prewarm_key(key) else {
                skipped += 1;
                continue;
            };

            if optimizer.is_cached(&param).await {
                skipped += 1;
                continue;
            }

            match optimizer.optimize(param).await {
                Ok((_, cache_hit)) if cache_hit => skipped += 1,
                Ok((_, _)) => warmed += 1,
                Err(error) => {
                    failed += 1;
                    tracing::debug!(error = %error, "[rari] Image prewarm skipped");
                }
            }
        }

        for url in blur_urls {
            let Some(_guard) = try_claim_prewarm_key(blur_dedupe_key(&url)) else {
                skipped += 1;
                continue;
            };

            if optimizer.get_blur_data_url(&url).await.is_some() {
                skipped += 1;
                continue;
            }

            match optimizer.ensure_blur_data_url(&url).await {
                Ok(_) => warmed += 1,
                Err(error) => {
                    failed += 1;
                    tracing::debug!(error = %error, "[rari] Image blur prewarm skipped");
                }
            }
        }

        if warmed > 0 || failed > 0 {
            tracing::debug!(warmed, skipped, failed, "[rari] Image prewarm finished");
        }
    });
}

pub fn schedule_image_prewarm(state: &ServerState, html: &str) {
    let Some(optimizer) = state.image_optimizer.as_ref() else {
        return;
    };
    schedule_prewarm_from_html(Arc::clone(optimizer), html);
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use super::{extract_optimize_params_from_html, unique_source_urls};

    #[test]
    fn extracts_unique_optimize_params_from_html() {
        let html = r#"
          <img src="/_rari/image?url=%2Fhero.jpg&amp;w=640&amp;q=75&amp;f=avif" />
          <img srcset="/_rari/image?url=%2Fhero.jpg&w=640&q=75&f=avif 640w,
                       /_rari/image?url=%2Fhero.jpg&w=1080&q=75&f=avif 1080w" />
          <img src="/_rari/image?url=https%3A%2F%2Fcdn.example%2Fnew.jpg&w=750&q=75" />
        "#;

        let params = extract_optimize_params_from_html(html);
        assert_eq!(params.len(), 3);

        let remote = params.iter().find(|p| p.url.contains("cdn.example")).expect("remote");
        assert_eq!(remote.w, Some(750));
        assert_eq!(remote.q, 75);

        let hero_640 =
            params.iter().find(|p| p.url == "/hero.jpg" && p.w == Some(640)).expect("hero 640");
        assert_eq!(hero_640.f.as_deref(), Some("avif"));

        let blur_urls = unique_source_urls(&params);
        assert_eq!(blur_urls.len(), 2);
        assert!(blur_urls.iter().any(|url| url == "/hero.jpg"));
        assert!(blur_urls.iter().any(|url| url.contains("cdn.example")));
    }

    #[test]
    fn ignores_non_image_urls() {
        let html = r#"<a href="/docs?url=1">x</a><img src="/assets/photo.jpg" />"#;
        assert!(extract_optimize_params_from_html(html).is_empty());
    }
}
