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

    with_blur_placeholders(params, &mut seen)
}

fn with_blur_placeholders(
    mut params: Vec<OptimizeParams>,
    seen: &mut HashSet<String>,
) -> Vec<OptimizeParams> {
    let source_urls: Vec<String> = params.iter().map(|param| param.url.clone()).collect();
    let mut unique_urls = HashSet::new();

    for url in source_urls {
        if !unique_urls.insert(url.clone()) {
            continue;
        }
        let blur = ImageOptimizer::blur_placeholder_params(url);
        let key = optimize_dedupe_key(&blur);
        if seen.insert(key) {
            params.push(blur);
        }
    }

    params
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

pub fn schedule_prewarm_from_html(optimizer: Arc<ImageOptimizer>, html: &str) {
    let params = extract_optimize_params_from_html(html);
    if params.is_empty() {
        return;
    }

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
    use crate::server::image::{
        BLUR_PLACEHOLDER_QUALITY, BLUR_PLACEHOLDER_WIDTH, extract_optimize_params_from_html,
    };

    #[test]
    fn extracts_unique_optimize_params_from_html() {
        let html = r#"
          <img src="/_rari/image?url=%2Fhero.jpg&amp;w=640&amp;q=75&amp;f=avif" />
          <img srcset="/_rari/image?url=%2Fhero.jpg&w=640&q=75&f=avif 640w,
                       /_rari/image?url=%2Fhero.jpg&w=1080&q=75&f=avif 1080w" />
          <img src="/_rari/image?url=https%3A%2F%2Fcdn.example%2Fnew.jpg&w=750&q=75" />
        "#;

        let params = extract_optimize_params_from_html(html);
        assert_eq!(params.len(), 5);

        let remote = params.iter().find(|p| p.url.contains("cdn.example")).expect("remote");
        assert_eq!(remote.w, Some(750));
        assert_eq!(remote.q, 75);

        let hero_640 =
            params.iter().find(|p| p.url == "/hero.jpg" && p.w == Some(640)).expect("hero 640");
        assert_eq!(hero_640.f.as_deref(), Some("avif"));

        let hero_blur = params
            .iter()
            .find(|p| {
                p.url == "/hero.jpg"
                    && p.w == Some(BLUR_PLACEHOLDER_WIDTH)
                    && p.q == BLUR_PLACEHOLDER_QUALITY
            })
            .expect("hero blur");
        assert_eq!(hero_blur.f.as_deref(), Some("jpeg"));
    }

    #[test]
    fn ignores_non_image_urls() {
        let html = r#"<a href="/docs?url=1">x</a><img src="/assets/photo.jpg" />"#;
        assert!(extract_optimize_params_from_html(html).is_empty());
    }
}
