use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use deno_core::{ModuleCodeString, ModuleName};
use regex::Regex;
use serde::Serialize;

use super::{DEFAULT_IMAGE_QUALITY, ImageVariant};
use crate::runtime::transpile::maybe_transpile_source;

fn parse_decimal_u32(value: &str) -> Option<u32> {
    value.parse().ok()
}

fn parse_decimal_u8(value: &str) -> Option<u8> {
    value.parse().ok()
}

fn init_regex(pattern: &str) -> Regex {
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    Regex::new(pattern).expect("valid image scanner regex")
}

fn compile_alias_regex(pattern: &str) -> Option<Regex> {
    Regex::new(pattern).ok()
}

struct AliasPatterns {
    jsx_self_closing: Regex,
    jsx_opening: Regex,
    create_element: Regex,
}

#[derive(Default)]
struct AliasRegexCache {
    patterns: HashMap<String, AliasPatterns>,
}

impl AliasRegexCache {
    fn get_or_insert(&mut self, alias: &str) -> Option<&AliasPatterns> {
        if !self.patterns.contains_key(alias) {
            let Some(jsx_self_closing) = compile_alias_regex(&format!(r"<{alias}\s([^/>]+)/>"))
            else {
                tracing::warn!(alias = %alias, "image scanner: skipping invalid alias regex");
                return None;
            };
            let Some(jsx_opening) = compile_alias_regex(&format!(r"<{alias}\s([^>]+)>")) else {
                tracing::warn!(alias = %alias, "image scanner: skipping invalid alias regex");
                return None;
            };
            let Some(create_element) =
                compile_alias_regex(&format!(r"React\.createElement\(\s*{alias}\s*,\s*\{{"))
            else {
                tracing::warn!(alias = %alias, "image scanner: skipping invalid alias regex");
                return None;
            };
            self.patterns.insert(
                alias.to_string(),
                AliasPatterns { jsx_self_closing, jsx_opening, create_element },
            );
        }
        self.patterns.get(alias)
    }
}

static DEFAULT_IMPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r#"import\s+(\w+)\s+from\s+['"]rari/image['"]"#));

static NAMED_IMPORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    init_regex(
        r#"import\s+\{[^}]*\b(?:Image\s+as\s+(\w+)|Image)\b[^}]*\}\s+from\s+['"]rari/image['"]"#,
    )
});

static SAFE_IDENTIFIER_REGEX: LazyLock<Regex> = LazyLock::new(|| init_regex(r"^[A-Za-z_$][\w$]*$"));

static SRC_PROP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r#"(?:^|\s)src=\{?["']([^"']+)["']\}?|(?:^|\s)src=\{([^}]+)\}"#));

static WIDTH_PROP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|\s)width=\{?(\d+)\}?"));

static QUALITY_PROP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|\s)quality=\{?(\d+)\}?"));

static PRELOAD_TRUE_PROP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|\s)preload(?:=\{?true\}?|\s|/|$)"));

static PRELOAD_FALSE_PROP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|\s)preload=\{?false\}?"));

static CREATE_ELEMENT_SRC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r#"(?:^|[\s,])src:\s*["']([^"']+)["']"#));

static CREATE_ELEMENT_WIDTH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|[\s,])width:\s*(\d+)"));

static CREATE_ELEMENT_QUALITY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|[\s,])quality:\s*(\d+)"));

static CREATE_ELEMENT_PRELOAD_TRUE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|[\s,])preload:\s*(true|!0)"));

static CREATE_ELEMENT_PRELOAD_FALSE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| init_regex(r"(?:^|[\s,])preload:\s*(false|!1)"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ImageUsageManifest {
    pub images: Vec<ImageVariant>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ScanError {
    #[error("required source directory does not exist: {0}")]
    MissingSourceDir(PathBuf),

    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to scan {path}: {source}")]
    ScanPath {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "tsx" | "ts" | "jsx" | "js"))
}

fn parse_image_imports(content: &str) -> Vec<String> {
    let mut identifiers = Vec::new();

    for captures in DEFAULT_IMPORT_REGEX.captures_iter(content) {
        if let Some(identifier) = captures.get(1) {
            identifiers.push(identifier.as_str().to_string());
        }
    }

    for captures in NAMED_IMPORT_REGEX.captures_iter(content) {
        if let Some(alias) = captures.get(1) {
            identifiers.push(alias.as_str().to_string());
        } else {
            identifiers.push("Image".to_string());
        }
    }

    identifiers.sort_unstable();
    identifiers.dedup();
    identifiers
}

fn is_valid_src(src: &str) -> bool {
    (src.starts_with('/') || src.starts_with("http://") || src.starts_with("https://"))
        && !src.contains('{')
}

fn dedup_key(usage: &ImageVariant) -> String {
    format!(
        "{}:{}:{}",
        usage.src,
        usage.width.map_or_else(|| "auto".to_string(), |width| width.to_string()),
        usage.quality.unwrap_or(DEFAULT_IMAGE_QUALITY)
    )
}

fn add_image_to_map(usage: ImageVariant, images: &mut HashMap<String, ImageVariant>) {
    let key = dedup_key(&usage);
    let replace = match images.get(&key) {
        Some(existing) => usage.preload == Some(true) && existing.preload != Some(true),
        None => true,
    };

    if replace {
        images.insert(key, usage);
    }
}

fn parse_numeric_props(
    props_string: &str,
    width_regex: &Regex,
    quality_regex: &Regex,
    preload_true_regex: &Regex,
    preload_false_regex: &Regex,
) -> (Option<u32>, Option<u8>, bool) {
    let width = width_regex
        .captures(props_string)
        .and_then(|captures| captures.get(1))
        .and_then(|value| parse_decimal_u32(value.as_str()));

    let quality = quality_regex
        .captures(props_string)
        .and_then(|captures| captures.get(1))
        .and_then(|value| parse_decimal_u8(value.as_str()));

    let preload =
        preload_true_regex.is_match(props_string) && !preload_false_regex.is_match(props_string);

    (width, quality, preload)
}

fn parse_jsx_props(props_string: &str) -> Option<ImageVariant> {
    let src = SRC_PROP_REGEX
        .captures(props_string)
        .and_then(|captures| captures.get(1).or_else(|| captures.get(2)).map(|m| m.as_str()))?;

    if !is_valid_src(src) {
        return None;
    }

    let (width, quality, preload) = parse_numeric_props(
        props_string,
        &WIDTH_PROP_REGEX,
        &QUALITY_PROP_REGEX,
        &PRELOAD_TRUE_PROP_REGEX,
        &PRELOAD_FALSE_PROP_REGEX,
    );

    Some(ImageVariant { src: src.to_string(), width, quality, preload: Some(preload) })
}

fn parse_create_element_props(props_string: &str) -> Option<ImageVariant> {
    let src = CREATE_ELEMENT_SRC_REGEX
        .captures(props_string)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())?;

    if !is_valid_src(src) {
        return None;
    }

    let (width, quality, preload) = parse_numeric_props(
        props_string,
        &CREATE_ELEMENT_WIDTH_REGEX,
        &CREATE_ELEMENT_QUALITY_REGEX,
        &CREATE_ELEMENT_PRELOAD_TRUE_REGEX,
        &CREATE_ELEMENT_PRELOAD_FALSE_REGEX,
    );

    Some(ImageVariant { src: src.to_string(), width, quality, preload: Some(preload) })
}

fn extract_balanced_braces(code: &str, start_index: usize) -> Option<String> {
    let mut brace_count = 0;
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escaped = false;
    let mut template_depth = 0;

    for (offset, ch) in code[start_index..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if !in_string && (ch == '"' || ch == '\'' || ch == '`') {
            in_string = true;
            string_char = ch;
            if ch == '`' {
                template_depth = 1;
            }
            continue;
        }

        if in_string && string_char == '`' && ch == '`' {
            // Inside ${ ... }, a backtick opens or closes a nested template literal.
            if brace_count > 1 && template_depth < brace_count {
                template_depth += 1;
                continue;
            }

            template_depth -= 1;
            if template_depth == 0 {
                in_string = false;
                string_char = '\0';
            }
            continue;
        }

        if in_string && ch == string_char {
            in_string = false;
            string_char = '\0';
            continue;
        }

        if in_string && string_char == '`' && ch == '$' {
            let next_index = start_index + offset + ch.len_utf8();
            if next_index < code.len() && code.as_bytes()[next_index] == b'{' {
                brace_count += 1;
                continue;
            }
        }

        if in_string && string_char == '`' && ch == '}' && brace_count > 0 {
            brace_count -= 1;
            continue;
        }

        if !in_string {
            if ch == '{' {
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    let end_index = start_index + offset;
                    return Some(code[start_index + 1..end_index].to_string());
                }
            }
        }
    }

    None
}

fn process_jsx_aliases(
    content: &str,
    aliases: &[String],
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    for alias in aliases {
        if !SAFE_IDENTIFIER_REGEX.is_match(alias) {
            tracing::warn!(alias = %alias, "image scanner: skipping unsafe alias");
            continue;
        }

        let Some(patterns) = alias_cache.get_or_insert(alias) else {
            continue;
        };

        for captures in patterns.jsx_self_closing.captures_iter(content) {
            if let Some(props) = captures.get(1)
                && let Some(usage) = parse_jsx_props(props.as_str())
            {
                add_image_to_map(usage, images);
            }
        }

        for captures in patterns.jsx_opening.captures_iter(content) {
            if let Some(props) = captures.get(1)
                && let Some(usage) = parse_jsx_props(props.as_str())
            {
                add_image_to_map(usage, images);
            }
        }
    }
}

fn process_create_element_aliases(
    transformed_code: &str,
    aliases: &[String],
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    for alias in aliases {
        if !SAFE_IDENTIFIER_REGEX.is_match(alias) {
            tracing::warn!(alias = %alias, "image scanner: skipping unsafe identifier");
            continue;
        }

        let Some(patterns) = alias_cache.get_or_insert(alias) else {
            continue;
        };

        for captures in patterns.create_element.captures_iter(transformed_code) {
            let Some(full_match) = captures.get(0) else {
                continue;
            };

            let brace_start = full_match.end() - 1;
            if let Some(props_string) = extract_balanced_braces(transformed_code, brace_start)
                && let Some(usage) = parse_create_element_props(&props_string)
            {
                add_image_to_map(usage, images);
            }
        }
    }
}

fn extract_image_usages(
    content: &str,
    file_path: &Path,
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    let aliases = parse_image_imports(content);
    if aliases.is_empty() {
        return;
    }

    let module_name = ModuleName::from(file_path.to_string_lossy().into_owned());
    let transpiled =
        maybe_transpile_source(&module_name, ModuleCodeString::from(content.to_string()));

    match transpiled {
        Ok((transformed_code, _)) => {
            process_create_element_aliases(&transformed_code, &aliases, images, alias_cache);
            if transformed_code.as_str() == content {
                process_jsx_aliases(content, &aliases, images, alias_cache);
            }
        }
        Err(_) => process_jsx_aliases(content, &aliases, images, alias_cache),
    }
}

fn process_file(
    path: &Path,
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %error, "image scanner: failed to process file");
            return;
        }
    };

    extract_image_usages(&content, path, images, alias_cache);
}

fn scan_directory(
    dir: &Path,
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::warn!(path = %dir.display(), error = %error, "image scanner: failed to read directory");
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(path = %dir.display(), error = %error, "image scanner: failed to read directory entry");
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                tracing::warn!(path = %path.display(), error = %error, "image scanner: failed to read file type");
                continue;
            }
        };

        if file_type.is_dir() {
            if path.file_name().is_some_and(|name| name == "node_modules" || name == "dist") {
                continue;
            }
            scan_directory(&path, images, alias_cache);
        } else if is_source_file(&path) {
            process_file(&path, images, alias_cache);
        }
    }
}

fn scan_optional_directory(
    dir: &Path,
    images: &mut HashMap<String, ImageVariant>,
    alias_cache: &mut AliasRegexCache,
) {
    match fs::metadata(dir) {
        Ok(_) => scan_directory(dir, images, alias_cache),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::warn!(path = %dir.display(), error = %error, "image scanner: failed to scan directory");
        }
    }
}

#[expect(clippy::missing_errors_doc)]
pub fn scan_for_image_usage(
    src_dir: impl AsRef<Path>,
    additional_dirs: &[PathBuf],
) -> Result<ImageUsageManifest, ScanError> {
    let src_dir = src_dir.as_ref();
    if fs::metadata(src_dir).is_err() {
        return Err(ScanError::MissingSourceDir(src_dir.to_path_buf()));
    }

    let mut images = HashMap::new();
    let mut alias_cache = AliasRegexCache::default();
    scan_directory(src_dir, &mut images, &mut alias_cache);

    for dir in additional_dirs {
        scan_optional_directory(dir, &mut images, &mut alias_cache);
    }

    Ok(ImageUsageManifest { images: images.into_values().collect() })
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::{env, fs, process, slice};

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("rari-image-scan-test-{name}-{}", process::id()))
    }

    fn write_source(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).expect("write source file");
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn extract_balanced_braces_handles_simple_template_interpolation() {
        let code = "React.createElement(Image, { src: `/photo-${id}.jpg`, width: 800 })";
        let brace_start = code.find("{ src").expect("props object");
        let props = extract_balanced_braces(code, brace_start).expect("extract props");
        assert!(props.contains("`/photo-${id}.jpg`"));
        assert!(props.contains("width: 800"));
    }

    #[test]
    fn extract_balanced_braces_handles_nested_template_literals() {
        let code = "React.createElement(Image, { src: `prefix${`nested`}suffix`, width: 800 })";
        let brace_start = code.find("{ src").expect("props object");
        let props = extract_balanced_braces(code, brace_start).expect("extract props");
        assert!(props.contains("`prefix${`nested`}suffix`"));
        assert!(props.contains("width: 800"));
    }

    #[test]
    fn parse_create_element_props_static_src() {
        let usage = parse_create_element_props(r#"src: "/hero.jpg", width: 800, quality: 90"#)
            .expect("parse props");
        assert_eq!(usage.src, "/hero.jpg");
        assert_eq!(usage.width, Some(800));
        assert_eq!(usage.quality, Some(90));
    }

    #[test]
    fn parse_jsx_props_ignores_similar_prop_names() {
        assert!(
            parse_jsx_props(r#"data-src="/fake.jpg" maxWidth={800} preloadPriority"#).is_none()
        );
        let usage = parse_jsx_props(r#" src="/real.jpg" width={800} "#).expect("parse props");
        assert_eq!(usage.src, "/real.jpg");
        assert_eq!(usage.width, Some(800));
    }

    #[test]
    fn parse_create_element_props_ignores_similar_prop_names() {
        assert!(parse_create_element_props(r#"dataSrc: "/fake.jpg", maxWidth: 800"#).is_none());
        let usage =
            parse_create_element_props(r#" src: "/real.jpg", width: 800 "#).expect("parse props");
        assert_eq!(usage.src, "/real.jpg");
        assert_eq!(usage.width, Some(800));
    }

    #[test]
    fn missing_source_dir_errors() {
        let error = scan_for_image_usage("/does/not/exist", &[]).unwrap_err();
        assert!(matches!(error, ScanError::MissingSourceDir(_)));
    }

    #[test]
    fn empty_directory_returns_empty_manifest() {
        let dir = test_dir("empty");
        fs::create_dir_all(&dir).expect("create dir");
        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert!(manifest.images.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scans_default_import_jsx_in_js_files() {
        let dir = test_dir("default-import-js");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Component.js",
            r#"
import Image from 'rari/image'

export default function MyComponent() {
  return <Image src="/test.js.jpg" width={800} quality={90} preload />
}
"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].src, "/test.js.jpg");
        assert_eq!(manifest.images[0].width, Some(800));
        assert_eq!(manifest.images[0].quality, Some(90));
        assert_eq!(manifest.images[0].preload, Some(true));
        cleanup(&dir);
    }

    #[test]
    fn scans_default_import_jsx() {
        let dir = test_dir("default-import");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Component.tsx",
            r#"
import Image from 'rari/image'

export default function MyComponent() {
  return <Image src="/test.jpg" width={800} quality={90} preload />
}
"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].src, "/test.jpg");
        assert_eq!(manifest.images[0].width, Some(800));
        assert_eq!(manifest.images[0].quality, Some(90));
        assert_eq!(manifest.images[0].preload, Some(true));
        cleanup(&dir);
    }

    #[test]
    fn scans_named_import_alias() {
        let dir = test_dir("named-import");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Test.tsx",
            r#"
import { Image as Img } from 'rari/image'

export default function MyComponent() {
  return <Img src="/photo.png" width={600} />
}
"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].src, "/photo.png");
        assert_eq!(manifest.images[0].width, Some(600));
        cleanup(&dir);
    }

    #[test]
    fn skips_node_modules_and_dist() {
        let dir = test_dir("skip-dirs");
        fs::create_dir_all(dir.join("node_modules/pkg")).expect("node_modules");
        fs::create_dir_all(dir.join("dist")).expect("dist");
        fs::create_dir_all(dir.join("src")).expect("src");

        write_source(
            &dir.join("node_modules/pkg"),
            "Hidden.tsx",
            r#"import Image from 'rari/image'; export default () => <Image src="/hidden.jpg" />"#,
        );
        write_source(
            &dir.join("dist"),
            "Hidden.tsx",
            r#"import Image from 'rari/image'; export default () => <Image src="/hidden.jpg" />"#,
        );
        write_source(
            &dir.join("src"),
            "Visible.tsx",
            r#"import Image from 'rari/image'; export default () => <Image src="/visible.jpg" width={400} />"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].src, "/visible.jpg");
        cleanup(&dir);
    }

    #[test]
    fn scans_multiple_images_and_deduplicates() {
        let dir = test_dir("multiple");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Gallery.tsx",
            r#"
import Image from 'rari/image'

export default function Gallery() {
  return (
    <>
      <Image src="/img1.jpg" width={400} />
      <Image src="/img2.jpg" width={600} quality={80} />
      <Image src="/img3.jpg" preload />
      <Image src="/img1.jpg" width={400} />
    </>
  )
}
"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 3);
        cleanup(&dir);
    }

    #[test]
    fn skips_dynamic_src() {
        let dir = test_dir("dynamic-src");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Dynamic.tsx",
            r"
import Image from 'rari/image'

export default function DynamicImage({ src }) {
  return <Image src={src} width={800} />
}
",
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert!(manifest.images.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn handles_preload_false() {
        let dir = test_dir("preload-false");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "NoPreload.tsx",
            r#"
import Image from 'rari/image'

export default function NoPreload() {
  return <Image src="/test.jpg" width={800} preload={false} />
}
"#,
        );

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].preload, Some(false));
        cleanup(&dir);
    }

    #[test]
    fn scans_additional_directories() {
        let src = test_dir("additional-src");
        let extra = test_dir("additional-extra");
        fs::create_dir_all(&src).expect("create src");
        fs::create_dir_all(&extra).expect("create extra");

        write_source(
            &extra,
            "component.tsx",
            r#"
import Image from 'rari/image'

export default function AdditionalComponent() {
  return <Image src="/additional.jpg" width={1024} quality={85} />
}
"#,
        );

        let manifest = scan_for_image_usage(&src, slice::from_ref(&extra)).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        assert_eq!(manifest.images[0].src, "/additional.jpg");
        assert_eq!(manifest.images[0].width, Some(1024));
        assert_eq!(manifest.images[0].quality, Some(85));
        cleanup(&src);
        cleanup(&extra);
    }

    #[test]
    fn ignores_missing_additional_directories() {
        let src = test_dir("missing-extra-src");
        fs::create_dir_all(&src).expect("create src");
        let manifest =
            scan_for_image_usage(&src, &[PathBuf::from("/missing/additional")]).expect("scan");
        assert!(manifest.images.is_empty());
        cleanup(&src);
    }

    #[test]
    fn only_processes_source_extensions() {
        let dir = test_dir("extensions");
        fs::create_dir_all(&dir).expect("create dir");
        write_source(
            &dir,
            "Component.tsx",
            r#"import Image from 'rari/image'; export default () => <Image src="/test.jpg" />"#,
        );
        write_source(&dir, "styles.css", "body {}");
        write_source(&dir, "data.json", "{}");
        write_source(&dir, "README.md", "# docs");

        let manifest = scan_for_image_usage(&dir, &[]).expect("scan");
        assert_eq!(manifest.images.len(), 1);
        cleanup(&dir);
    }
}
