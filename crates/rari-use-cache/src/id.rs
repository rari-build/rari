use sha2::{Digest, Sha256};

pub fn generate_reference_id(
    hash_salt: &str,
    filename: &str,
    export_name: &str,
    is_cache: bool,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(hash_salt.as_bytes());
    hasher.update(filename.as_bytes());
    hasher.update(b":");
    hasher.update(export_name.as_bytes());

    let hash = hasher.finalize();

    let mut bytes = hash.to_vec();
    let type_byte = if is_cache { 0x01u8 } else { 0x00u8 };
    bytes.push(type_byte);

    hex::encode(bytes)
}

pub fn generate_cache_export_name(index: usize, export_name: &str) -> String {
    let sanitized = sanitize_export_name(export_name);
    let index_str = index.to_string();
    let mut result = String::with_capacity(20 + index_str.len() + sanitized.len());
    result.push_str("$$RSC_SERVER_CACHE_");
    result.push_str(&index_str);
    result.push('_');
    result.push_str(&sanitized);
    result
}

pub fn generate_cache_inner_name(index: usize, export_name: &str) -> String {
    let sanitized = sanitize_export_name(export_name);
    let index_str = index.to_string();
    let mut result = String::with_capacity(26 + index_str.len() + sanitized.len());
    result.push_str("$$RSC_SERVER_CACHE_");
    result.push_str(&index_str);
    result.push('_');
    result.push_str(&sanitized);
    result.push_str("_INNER");
    result
}

fn sanitize_export_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reference_id_is_hex() {
        let id = generate_reference_id("salt", "file.tsx", "getData", true);
        // SHA256 = 64 hex chars + 2 for type byte = 66 hex chars
        assert_eq!(id.len(), 66);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_reference_id_deterministic() {
        let id1 = generate_reference_id("salt", "file.tsx", "getData", true);
        let id2 = generate_reference_id("salt", "file.tsx", "getData", true);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_reference_id_different_salt() {
        let id1 = generate_reference_id("salt1", "file.tsx", "getData", true);
        let id2 = generate_reference_id("salt2", "file.tsx", "getData", true);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_reference_id_different_export() {
        let id1 = generate_reference_id("salt", "file.tsx", "getData", true);
        let id2 = generate_reference_id("salt", "file.tsx", "getOther", true);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_cache_export_name_format() {
        let name = generate_cache_export_name(0, "getData");
        assert_eq!(name, "$$RSC_SERVER_CACHE_0_getData");
    }

    #[test]
    fn test_cache_inner_name_format() {
        let name = generate_cache_inner_name(1, "fetchStuff");
        assert_eq!(name, "$$RSC_SERVER_CACHE_1_fetchStuff_INNER");
    }

    #[test]
    fn test_cache_export_name_sanitizes() {
        let name = generate_cache_export_name(0, "get-data");
        assert_eq!(name, "$$RSC_SERVER_CACHE_0_get_data");
    }
}
