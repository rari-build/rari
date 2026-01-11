use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use parking_lot::RwLock;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const SUSPICIOUS_REQUEST_THRESHOLD: u32 = 20;
const TIME_WINDOW_SECS: u64 = 3600;

#[derive(Debug, Clone)]
pub enum BlockReason {
    IpBlocked,
    PathPattern(String),
    UserAgent(String),
}

#[derive(Clone)]
pub struct SpamBlocker {
    blocked_patterns: Vec<Regex>,
    blocked_user_agents: Vec<Regex>,
    ip_tracker: Arc<RwLock<FxHashMap<String, IpData>>>,
    blocked_ips: Arc<RwLock<FxHashSet<String>>>,
}

#[derive(Debug, Clone)]
struct IpData {
    count: u32,
    first_seen: u64,
}

impl SpamBlocker {
    pub fn new() -> Self {
        let blocked_patterns = vec![
            Regex::new(r"(?i)\.php($|/|\?)").expect("Valid regex pattern"),
            Regex::new(r"(?i)\.(aspx?|ashx)($|/|\?)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/wp-").expect("Valid regex pattern"),
            Regex::new(r"(?i)/wordpress").expect("Valid regex pattern"),
            Regex::new(r"(?i)/xmlrpc").expect("Valid regex pattern"),
            Regex::new(r"(?i)/wp-content/themes/").expect("Valid regex pattern"),
            Regex::new(r"(?i)/wp-content/plugins/").expect("Valid regex pattern"),
            Regex::new(r"(?i)/theme/[^/]+/assets/").expect("Valid regex pattern"),
            Regex::new(r"(?i)/(laravel|artisan|_ignition|telescope|horizon)")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)/(symfony|drupal|joomla|magento)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.(env|git|svn|hg|bzr)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.(vscode|idea|DS_Store)").expect("Valid regex pattern"),
            Regex::new(r"(?i)\.(swp|swo)$").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.vscode/").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.idea/").expect("Valid regex pattern"),
            Regex::new(r"(?i)/sftp-config\.json").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.ftpconfig").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.remote-sync\.json").expect("Valid regex pattern"),
            Regex::new(r"(?i)/deployment\.xml").expect("Valid regex pattern"),
            Regex::new(r"(?i)/(phpmyadmin|adminer|pgadmin|mongo-express)")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)/administrator").expect("Valid regex pattern"),
            Regex::new(r"(?i)/\.ht(access|passwd)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/(config|configuration|settings|database)\.")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)\.(sql|zip|tar|gz|bak|backup|old|orig|save)($|\.)")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)(\.log$|/logs/)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/(cgi-bin|phpunit|phpinfo|shell|webshell)")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)/(server-status|server-info)").expect("Valid regex pattern"),
            Regex::new(r"(?i)/api/v1/guest/comm/config").expect("Valid regex pattern"),
            Regex::new(r"(?i)/vendor/phpunit").expect("Valid regex pattern"),
        ];

        let blocked_user_agents = vec![
            Regex::new(r"(?i)(masscan|sqlmap|nikto|nmap|acunetix|nessus)")
                .expect("Valid regex pattern"),
            Regex::new(r"(?i)(havij|metasploit|burp)").expect("Valid regex pattern"),
            Regex::new(r"(?i)(zgrab|shodan)").expect("Valid regex pattern"),
        ];

        Self {
            blocked_patterns,
            blocked_user_agents,
            ip_tracker: Arc::new(RwLock::new(FxHashMap::default())),
            blocked_ips: Arc::new(RwLock::new(FxHashSet::default())),
        }
    }

    pub fn is_blocked(&self, path: &str, user_agent: &str, ip: &str) -> bool {
        self.check_blocked(path, user_agent, ip).is_some()
    }

    pub fn check_blocked(&self, path: &str, user_agent: &str, ip: &str) -> Option<BlockReason> {
        if self.blocked_ips.read().contains(ip) {
            return Some(BlockReason::IpBlocked);
        }

        for pattern in &self.blocked_patterns {
            if pattern.is_match(path) {
                self.track_suspicious_ip(ip, path);
                return Some(BlockReason::PathPattern(pattern.to_string()));
            }
        }

        for pattern in &self.blocked_user_agents {
            if pattern.is_match(user_agent) {
                self.track_suspicious_ip(ip, path);
                return Some(BlockReason::UserAgent(pattern.to_string()));
            }
        }

        None
    }

    fn track_suspicious_ip(&self, ip: &str, _path: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let mut tracker = self.ip_tracker.write();

        let ip_data = tracker.entry(ip.to_string()).or_insert(IpData { count: 0, first_seen: now });

        if now - ip_data.first_seen > TIME_WINDOW_SECS {
            ip_data.count = 1;
            ip_data.first_seen = now;
        } else {
            ip_data.count += 1;
        }

        if ip_data.count > SUSPICIOUS_REQUEST_THRESHOLD {
            drop(tracker);
            self.block_ip(ip);
        }
    }

    pub fn block_ip(&self, ip: &str) {
        self.blocked_ips.write().insert(ip.to_string());
    }

    #[allow(dead_code)]
    pub fn unblock_ip(&self, ip: &str) {
        self.blocked_ips.write().remove(ip);
    }

    pub fn cleanup_old_records(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let mut tracker = self.ip_tracker.write();
        tracker.retain(|_, data| now - data.first_seen <= TIME_WINDOW_SECS);
    }

    pub fn start_cleanup_task(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                self.cleanup_old_records();
            }
        });
    }
}

impl Default for SpamBlocker {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn spam_blocker_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let spam_blocker = req
        .extensions()
        .get::<SpamBlocker>()
        .cloned()
        .expect("SpamBlocker middleware requires SpamBlocker in request extensions");

    let path = req.uri().path();
    let user_agent = req.headers().get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("");
    let ip = addr.ip().to_string();

    if let Some(reason) = spam_blocker.check_blocked(path, user_agent, &ip) {
        #[cfg(debug_assertions)]
        eprintln!("[spam_blocker] Blocked {} from {}: {:?}", path, ip, reason);

        req.extensions_mut().insert(SpamRequest);

        return Ok(SpamBlockedResponse.into_response());
    }

    Ok(next.run(req).await)
}

struct SpamBlockedResponse;

impl IntoResponse for SpamBlockedResponse {
    fn into_response(self) -> Response {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

#[derive(Clone)]
pub struct SpamRequest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_wordpress_paths() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/wp-admin/index.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/wp-content/uploads/shell.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/xmlrpc.php", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_php_files() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/index.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/admin.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/config.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/test.php?id=1", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_laravel_paths() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/laravel", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/_ignition/health-check", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/telescope", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/horizon", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_env_files() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/.env", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.env.local", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.git/config", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.svn/entries", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_aspnet_files() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/default.aspx", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/handler.ashx", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_config_and_backup_files() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/config.php", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/database.yml", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/backup.sql", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/site.tar.gz", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/app.bak", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_suspicious_user_agents() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/", "sqlmap/1.0", "127.0.0.1"));
        assert!(blocker.is_blocked("/", "nikto scanner", "127.0.0.1"));
        assert!(blocker.is_blocked("/", "Shodan/1.0", "127.0.0.1"));
        assert!(blocker.is_blocked("/", "zgrab/0.x", "127.0.0.1"));
    }

    #[test]
    fn test_allows_legitimate_requests() {
        let blocker = SpamBlocker::new();
        assert!(!blocker.is_blocked("/", "Mozilla/5.0", "127.0.0.1"));
        assert!(!blocker.is_blocked("/api/users", "Mozilla/5.0", "127.0.0.1"));
        assert!(!blocker.is_blocked("/about", "Chrome/120.0", "127.0.0.1"));
        assert!(!blocker.is_blocked("/blog/post-123", "Safari/17.0", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_wordpress_themes() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/wp-content/themes/twentytwenty/style.css", "", "127.0.0.1"));
        assert!(blocker.is_blocked(
            "/wp-content/themes/default/assets/js/main.js",
            "",
            "127.0.0.1"
        ));
        assert!(blocker.is_blocked("/theme/default/assets/components.js", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/theme/custom/assets/style.css", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/wp-content/plugins/akismet/akismet.php", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_vscode_config() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/.vscode/settings.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.vscode/launch.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.vscode/sftp.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.idea/workspace.xml", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.idea/modules.xml", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_sftp_config() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/sftp-config.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.ftpconfig", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/.remote-sync.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/deployment.xml", "", "127.0.0.1"));
    }

    #[test]
    fn test_blocks_ide_files_in_subdirs() {
        let blocker = SpamBlocker::new();
        assert!(blocker.is_blocked("/project/.vscode/settings.json", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/app/.idea/workspace.xml", "", "127.0.0.1"));
        assert!(blocker.is_blocked("/src/sftp-config.json", "", "127.0.0.1"));
    }

    #[test]
    fn test_ip_blocking() {
        let blocker = SpamBlocker::new();
        blocker.block_ip("192.168.1.1");
        assert!(blocker.is_blocked("/", "Mozilla/5.0", "192.168.1.1"));
    }

    #[test]
    fn test_check_blocked_returns_reason() {
        let blocker = SpamBlocker::new();

        blocker.block_ip("10.0.0.1");
        let reason = blocker.check_blocked("/", "Mozilla/5.0", "10.0.0.1");
        assert!(matches!(reason, Some(BlockReason::IpBlocked)));

        let reason = blocker.check_blocked("/admin.php", "Mozilla/5.0", "127.0.0.1");
        assert!(matches!(reason, Some(BlockReason::PathPattern(_))));

        let reason = blocker.check_blocked("/", "sqlmap/1.0", "127.0.0.2");
        assert!(matches!(reason, Some(BlockReason::UserAgent(_))));

        let reason = blocker.check_blocked("/", "Mozilla/5.0", "127.0.0.3");
        assert!(reason.is_none());
    }

    #[test]
    fn test_legitimate_homepage_not_blocked() {
        let blocker = SpamBlocker::new();
        assert!(!blocker.is_blocked(
            "/",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
            "192.168.1.100"
        ));
        assert!(!blocker.is_blocked(
            "/",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0",
            "10.0.0.50"
        ));
        assert!(blocker.check_blocked("/", "Mozilla/5.0", "8.8.8.8").is_none());
    }
}
