use sha1::{Digest, Sha1};

pub fn generate_reference_id(
    hash_salt: &str,
    filename: &str,
    export_name: &str,
    is_cache: bool,
) -> String {
    let mut hasher = Sha1::new();
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
    format!("$$RSC_SERVER_CACHE_{}_{}", index, sanitize_export_name(export_name))
}

pub fn generate_cache_inner_name(index: usize, export_name: &str) -> String {
    format!("$$RSC_SERVER_CACHE_{}_{}_INNER", index, sanitize_export_name(export_name))
}

fn sanitize_export_name(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reference_id_is_hex() {
        let id = generate_reference_id("salt", "file.tsx", "getData", true);
        // SHA1 = 40 hex chars + 2 for type byte = 42 hex chars
        assert_eq!(id.len(), 42);
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
