use std::{
    borrow::Cow,
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use parley::fontique::Blob;
use reqwest::blocking::{Client, Response};
use sha2::{Digest, Sha256};

use super::fonts::{has_usable_face, load_font};

const FONT_PATH_ENV: &str = "RARI_OG_CJK_FONT";

const FONT_URL: &str = "https://raw.githubusercontent.com/google/fonts/3be1884c48c3e45b52ecc725676a08f87776373e/ofl/notosanstc/NotoSansTC%5Bwght%5D.ttf";
const FONT_SHA256: &str = "864727d210d54f2537bbe23b3a839436c3992af72de9322af5270897246bd44f";
const MAX_FONT_BYTES: u64 = 16 * 1024 * 1024;

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const RETRY_AFTER: Duration = Duration::from_secs(60);

#[derive(Default)]
struct State {
    font: Option<Blob<u8>>,
    last_failure: Option<Instant>,
}

static STATE: Mutex<State> = Mutex::new(State { font: None, last_failure: None });

pub fn is_needed(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            u32::from(c),
            0x2E80..=0xABFF
                | 0xF900..=0xFAFF
                | 0xFE10..=0xFEFE
                | 0xFF00..=0xFFEF
                | 0x1F100..=0x1F16F
                | 0x20000..=0x3FFFF
        )
    })
}

pub fn load() -> Option<Blob<u8>> {
    let mut state = STATE.lock();

    if let Some(font) = &state.font {
        return Some(font.clone());
    }
    if state.last_failure.is_some_and(|at| at.elapsed() < RETRY_AFTER) {
        return None;
    }

    match resolve() {
        Ok(data) => {
            let blob = Blob::from(data);
            state.font = Some(blob.clone());
            Some(blob)
        }
        Err(error) => {
            tracing::warn!(
                "Could not load the CJK font for OG images; CJK text will be missing. \
                 Set {FONT_PATH_ENV} to a local font to avoid the download. {error}"
            );
            state.last_failure = Some(Instant::now());
            None
        }
    }
}

fn resolve() -> Result<Vec<u8>, String> {
    if let Some(path) = env::var_os(FONT_PATH_ENV).filter(|path| !path.is_empty()) {
        let display = Path::new(&path).display();
        let data = fs::read(&path).map_err(|e| format!("{display}: {e}"))?;
        let data = load_font(&data).map(Cow::into_owned).map_err(|e| format!("{display}: {e}"))?;
        if !has_usable_face(&data) {
            return Err(format!("{display}: no usable font face"));
        }
        return Ok(data);
    }

    let cache_path = cache_path();
    if let Ok(data) = fs::read(&cache_path)
        && is_expected(&data)
    {
        return Ok(data);
    }

    let data = download()?;
    if let Err(error) = write_cache(&cache_path, &data) {
        tracing::debug!("Could not cache the CJK font at {}: {error}", cache_path.display());
    }
    Ok(data)
}

fn download() -> Result<Vec<u8>, String> {
    let client = Client::builder().timeout(DOWNLOAD_TIMEOUT).build().map_err(|e| e.to_string())?;
    let response = client
        .get(FONT_URL)
        .send()
        .and_then(Response::error_for_status)
        .map_err(|e| e.to_string())?;

    let mut data = Vec::new();
    response.take(MAX_FONT_BYTES).read_to_end(&mut data).map_err(|e| e.to_string())?;

    if is_expected(&data) { Ok(data) } else { Err(format!("{FONT_URL}: checksum mismatch")) }
}

fn is_expected(data: &[u8]) -> bool {
    hex::encode(Sha256::digest(data)) == FONT_SHA256
}

fn cache_path() -> PathBuf {
    env::temp_dir().join("rari-fonts").join(format!("{FONT_SHA256}.ttf"))
}

fn write_cache(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension(format!("{}.partial", process::id()));
    fs::write(&partial, data)?;
    fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needed_only_for_text_outside_the_subset() {
        for text in
            ["", "Hello, world", "Ünïcödé", "Привет", "Ελληνικά", "a → b", "🎉 ❤️ 🇺🇸", "안녕"]
        {
            assert!(!is_needed(text), "{text}");
        }
        for text in ["漢字", "ひらがな", "カタカナ", "ㄅㄆㄇ", "Ａ", "Hi 世界", "𠮷"]
        {
            assert!(is_needed(text), "{text}");
        }
    }

    #[test]
    fn rejects_unexpected_data() {
        assert!(!is_expected(b"not a font"));
    }

    #[test]
    fn rejects_local_font_without_faces() {
        assert!(!has_usable_face(&[0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0]));
    }
}
