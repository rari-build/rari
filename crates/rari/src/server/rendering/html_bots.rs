use std::sync::LazyLock;

use regex::Regex;

pub const HTML_LIMITED_BOT_UA_RE: &str = r"[\w-]+-Google|Google-[\w-]+|Chrome-Lighthouse|Slurp|DuckDuckBot|baiduspider|yandex|sogou|bitlybot|tumblr|vkShare|quora link preview|redditbot|ia_archiver|Bingbot|BingPreview|applebot|facebookexternalhit|facebookcatalog|Twitterbot|LinkedInBot|Slackbot|Discordbot|WhatsApp|SkypeUriPreview|Yeti|googleweblight";

static DEFAULT_HTML_LIMITED_BOTS: LazyLock<Regex> = LazyLock::new(|| {
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    Regex::new(HTML_LIMITED_BOT_UA_RE).expect("default htmlLimitedBots regex is valid")
});

/// Validate an `htmlLimitedBots` override pattern. Returns `Ok` when it compiles.
///
/// # Errors
///
/// Returns the underlying [`regex::Error`] when `pattern` is not a valid regex.
pub fn validate_html_limited_bots_pattern(pattern: &str) -> Result<(), regex::Error> {
    Regex::new(pattern).map(|_| ())
}

pub fn is_html_limited_bot(user_agent: Option<&str>, override_pattern: Option<&str>) -> bool {
    let Some(ua) = user_agent.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };

    match override_pattern {
        Some(pattern) => match Regex::new(pattern) {
            Ok(re) => re.is_match(ua),
            Err(err) => {
                tracing::warn!(
                    "Invalid htmlLimitedBots regex {pattern:?}: {err}. Falling back to default list."
                );
                DEFAULT_HTML_LIMITED_BOTS.is_match(ua)
            }
        },
        None => DEFAULT_HTML_LIMITED_BOTS.is_match(ua),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_social_and_search_bots() {
        assert!(is_html_limited_bot(Some("Twitterbot/1.0"), None));
        assert!(is_html_limited_bot(Some("Slackbot-LinkExpanding 1.0"), None));
        assert!(is_html_limited_bot(Some("facebookexternalhit/1.1"), None));
        assert!(is_html_limited_bot(Some("LinkedInBot/1.0"), None));
        assert!(is_html_limited_bot(Some("Bingbot/2.0"), None));
        assert!(is_html_limited_bot(
            Some("AdsBot-Google (+http://www.google.com/adsbot.html)"),
            None
        ));
        assert!(is_html_limited_bot(Some("Google-InspectionTool"), None));
        assert!(is_html_limited_bot(Some("Chrome-Lighthouse"), None));
    }

    #[test]
    fn allows_browsers_and_googlebot() {
        assert!(!is_html_limited_bot(
            Some("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)"),
            None
        ));
        assert!(!is_html_limited_bot(
            Some(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0"
            ),
            None
        ));
        assert!(!is_html_limited_bot(None, None));
        assert!(!is_html_limited_bot(Some(""), None));
    }

    #[test]
    fn override_replaces_default_list() {
        assert!(!is_html_limited_bot(Some("Twitterbot/1.0"), Some(r"OnlyMyBot")));
        assert!(is_html_limited_bot(Some("OnlyMyBot/2.0"), Some(r"OnlyMyBot")));
    }

    #[test]
    fn invalid_override_falls_back_to_default() {
        assert!(is_html_limited_bot(Some("Twitterbot/1.0"), Some(r"(OnlyMyBot")));
        assert!(validate_html_limited_bots_pattern(r"(OnlyMyBot").is_err());
        assert!(validate_html_limited_bots_pattern(r"OnlyMyBot").is_ok());
    }
}
