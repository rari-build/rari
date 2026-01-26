#[cfg(test)]
#[allow(clippy::disallowed_methods)]
#[allow(clippy::module_inception)]
#[allow(clippy::bool_assert_comparison)]
#[allow(clippy::approx_constant)]
mod tests {
    use crate::server::actions::{
        ValidationConfig, is_dangerous_property, validate_and_sanitize_args,
    };
    use serde_json::json;

    #[test]
    fn test_sanitize_args_removes_proto() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "__proto__": {
                "isAdmin": true
            },
            "username": "test"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        assert_eq!(sanitized.len(), 1);
        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("__proto__"));
        assert_eq!(obj.get("username").unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn test_sanitize_args_removes_constructor() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "constructor": {
                "prototype": {
                    "isAdmin": true
                }
            },
            "data": "safe"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("constructor"));
        assert_eq!(obj.get("data").unwrap().as_str().unwrap(), "safe");
    }

    #[test]
    fn test_sanitize_args_removes_prototype() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "prototype": {
                "polluted": true
            },
            "normal": "value"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("prototype"));
        assert_eq!(obj.get("normal").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_sanitize_args_nested_objects() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "user": {
                "__proto__": {
                    "isAdmin": true
                },
                "name": "John",
                "settings": {
                    "constructor": "bad",
                    "theme": "dark"
                }
            }
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        let user = obj.get("user").unwrap().as_object().unwrap();
        assert!(!user.contains_key("__proto__"));
        assert_eq!(user.get("name").unwrap().as_str().unwrap(), "John");

        let settings = user.get("settings").unwrap().as_object().unwrap();
        assert!(!settings.contains_key("constructor"));
        assert_eq!(settings.get("theme").unwrap().as_str().unwrap(), "dark");
    }

    #[test]
    fn test_sanitize_args_arrays() {
        let config = ValidationConfig::default();
        let args = vec![json!([
            {
                "__proto__": "bad",
                "id": 1
            },
            {
                "constructor": "bad",
                "id": 2
            }
        ])];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let arr = sanitized[0].as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let obj1 = arr[0].as_object().unwrap();
        assert!(!obj1.contains_key("__proto__"));
        assert_eq!(obj1.get("id").unwrap().as_i64().unwrap(), 1);

        let obj2 = arr[1].as_object().unwrap();
        assert!(!obj2.contains_key("constructor"));
        assert_eq!(obj2.get("id").unwrap().as_i64().unwrap(), 2);
    }

    #[test]
    fn test_sanitize_args_preserves_safe_data() {
        let config = ValidationConfig::default();
        let args = vec![
            json!("string value"),
            json!(42),
            json!(true),
            json!(null),
            json!({
                "name": "test",
                "count": 10,
                "active": true,
                "tags": ["a", "b", "c"]
            }),
        ];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        assert_eq!(sanitized.len(), 5);
        assert_eq!(sanitized[0].as_str().unwrap(), "string value");
        assert_eq!(sanitized[1].as_i64().unwrap(), 42);
        assert_eq!(sanitized[2].as_bool().unwrap(), true);
        assert!(sanitized[3].is_null());

        let obj = sanitized[4].as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "test");
        assert_eq!(obj.get("count").unwrap().as_i64().unwrap(), 10);
        assert_eq!(obj.get("active").unwrap().as_bool().unwrap(), true);
        assert_eq!(obj.get("tags").unwrap().as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_is_dangerous_property() {
        assert!(is_dangerous_property("__proto__"));
        assert!(is_dangerous_property("constructor"));
        assert!(is_dangerous_property("prototype"));
        assert!(is_dangerous_property("__defineGetter__"));
        assert!(is_dangerous_property("__defineSetter__"));
        assert!(is_dangerous_property("__lookupGetter__"));
        assert!(is_dangerous_property("__lookupSetter__"));

        assert!(!is_dangerous_property("name"));
        assert!(!is_dangerous_property("value"));
        assert!(!is_dangerous_property("data"));
        assert!(!is_dangerous_property("__typename"));
    }

    #[test]
    fn test_sanitize_deeply_nested() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "__proto__": "bad",
                        "level4": {
                            "constructor": "bad",
                            "safe": "value"
                        }
                    }
                }
            }
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        let level1 = obj.get("level1").unwrap().as_object().unwrap();
        let level2 = level1.get("level2").unwrap().as_object().unwrap();
        let level3 = level2.get("level3").unwrap().as_object().unwrap();
        assert!(!level3.contains_key("__proto__"));

        let level4 = level3.get("level4").unwrap().as_object().unwrap();
        assert!(!level4.contains_key("constructor"));
        assert_eq!(level4.get("safe").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_validation_depth_limit() {
        let config = ValidationConfig { max_depth: 3, ..Default::default() };

        let valid = vec![json!({
            "level1": {
                "level2": {
                    "level3": "ok"
                }
            }
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "too deep"
                    }
                }
            }
        })];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nesting depth"));
    }

    #[test]
    fn test_validation_string_length() {
        let config = ValidationConfig { max_string_length: 100, ..Default::default() };

        let valid = vec![json!({"text": "A".repeat(100)})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({"text": "A".repeat(101)})];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("String too long"));
    }

    #[test]
    fn test_validation_array_length() {
        let config = ValidationConfig { max_array_length: 10, ..Default::default() };

        let valid = vec![json!({"items": vec![1; 10]})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({"items": vec![1; 11]})];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Array too large"));
    }

    #[test]
    fn test_validation_object_keys() {
        let config = ValidationConfig { max_object_keys: 5, ..Default::default() };

        let mut valid_obj = serde_json::Map::new();
        for i in 0..5 {
            valid_obj.insert(format!("key{}", i), json!(i));
        }
        let valid = vec![json!(valid_obj)];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let mut invalid_obj = serde_json::Map::new();
        for i in 0..6 {
            invalid_obj.insert(format!("key{}", i), json!(i));
        }
        let invalid = vec![json!(invalid_obj)];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many object keys"));
    }

    #[test]
    fn test_validation_special_numbers() {
        let config = ValidationConfig { allow_special_numbers: false, ..Default::default() };

        let valid = vec![json!({"value": 42.5})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let valid_negative = vec![json!({"value": -123.456})];
        assert!(validate_and_sanitize_args(&valid_negative, &config).is_ok());
    }

    #[test]
    fn test_validation_combined_limits() {
        let config = ValidationConfig {
            max_depth: 3,
            max_string_length: 50,
            max_array_length: 3,
            max_object_keys: 3,
            allow_special_numbers: false,
            max_total_elements: 100_000,
        };

        let valid = vec![json!({
            "user": {
                "name": "John",
                "tags": ["a", "b", "c"]
            }
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let too_deep = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "fail"
                    }
                }
            }
        })];
        assert!(validate_and_sanitize_args(&too_deep, &config).is_err());

        let long_string = vec![json!({
            "text": "A".repeat(51)
        })];
        assert!(validate_and_sanitize_args(&long_string, &config).is_err());

        let large_array = vec![json!({
            "items": vec![1, 2, 3, 4]
        })];
        assert!(validate_and_sanitize_args(&large_array, &config).is_err());

        let many_keys = vec![json!({
            "key1": 1,
            "key2": 2,
            "key3": 3,
            "key4": 4
        })];
        assert!(validate_and_sanitize_args(&many_keys, &config).is_err());
    }

    #[test]
    fn test_validation_with_dangerous_properties() {
        let config = ValidationConfig::default();

        let args = vec![json!({
            "__proto__": {"isAdmin": true},
            "username": "test",
            "data": "A".repeat(100)
        })];

        let result = validate_and_sanitize_args(&args, &config).unwrap();
        let obj = result[0].as_object().unwrap();

        assert!(!obj.contains_key("__proto__"));
        assert_eq!(obj.get("username").unwrap().as_str().unwrap(), "test");
        assert_eq!(obj.get("data").unwrap().as_str().unwrap().len(), 100);
    }

    #[test]
    fn test_validation_nested_arrays() {
        let config = ValidationConfig { max_depth: 3, max_array_length: 2, ..Default::default() };

        let valid = vec![json!({
            "matrix": [
                [1, 2],
                [3, 4]
            ]
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({
            "matrix": [
                [1, 2, 3]
            ]
        })];
        assert!(validate_and_sanitize_args(&invalid, &config).is_err());
    }

    #[test]
    fn test_validation_preserves_types() {
        let config = ValidationConfig::default();

        let args = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(-123),
            json!(3.14),
            json!("string"),
            json!([1, 2, 3]),
            json!({"key": "value"}),
        ];

        let result = validate_and_sanitize_args(&args, &config).unwrap();

        assert!(result[0].is_null());
        assert_eq!(result[1].as_bool().unwrap(), true);
        assert_eq!(result[2].as_bool().unwrap(), false);
        assert_eq!(result[3].as_i64().unwrap(), 42);
        assert_eq!(result[4].as_i64().unwrap(), -123);
        assert_eq!(result[5].as_f64().unwrap(), 3.14);
        assert_eq!(result[6].as_str().unwrap(), "string");
        assert_eq!(result[7].as_array().unwrap().len(), 3);
        assert_eq!(result[8].as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_validation_config_development() {
        let dev_config = ValidationConfig::development();

        assert_eq!(dev_config.max_depth, 20);
        assert_eq!(dev_config.max_string_length, 50_000);
        assert_eq!(dev_config.max_array_length, 5_000);
        assert_eq!(dev_config.max_object_keys, 500);
    }

    #[test]
    fn test_validation_config_production() {
        let prod_config = ValidationConfig::production();

        assert_eq!(prod_config.max_depth, 10);
        assert_eq!(prod_config.max_string_length, 10_000);
        assert_eq!(prod_config.max_array_length, 1_000);
        assert_eq!(prod_config.max_object_keys, 100);
    }

    #[test]
    fn test_validation_empty_structures() {
        let config = ValidationConfig::default();

        let empty_obj = vec![json!({})];
        assert!(validate_and_sanitize_args(&empty_obj, &config).is_ok());

        let empty_arr = vec![json!([])];
        assert!(validate_and_sanitize_args(&empty_arr, &config).is_ok());

        let empty_str = vec![json!({"text": ""})];
        assert!(validate_and_sanitize_args(&empty_str, &config).is_ok());
    }

    #[test]
    fn test_validation_realistic_payload() {
        let config = ValidationConfig::default();

        let args = vec![json!({
            "user": {
                "id": 123,
                "name": "John Doe",
                "email": "john@example.com",
                "roles": ["user", "admin"],
                "metadata": {
                    "lastLogin": "2025-12-09T14:00:00Z",
                    "preferences": {
                        "theme": "dark",
                        "notifications": true
                    }
                }
            },
            "action": "update",
            "timestamp": 1733756400
        })];

        let result = validate_and_sanitize_args(&args, &config);
        assert!(result.is_ok());

        let sanitized = result.unwrap();
        assert_eq!(sanitized.len(), 1);

        let obj = sanitized[0].as_object().unwrap();
        assert!(obj.contains_key("user"));
        assert!(obj.contains_key("action"));
        assert!(obj.contains_key("timestamp"));
    }

    use crate::server::actions::validate_redirect_url;
    use crate::server::config::RedirectConfig;

    #[test]
    fn test_redirect_relative_url_allowed() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("/dashboard", &config).is_ok());
        assert!(validate_redirect_url("/users/123", &config).is_ok());
        assert!(validate_redirect_url("/", &config).is_ok());
    }

    #[test]
    fn test_redirect_relative_url_blocked_when_disabled() {
        let config = RedirectConfig {
            allowed_hosts: vec![],
            allow_relative: false,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("/dashboard", &config).is_err());
    }

    #[test]
    fn test_redirect_protocol_relative_blocked() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("//evil.com/phishing", &config).is_err());
    }

    #[test]
    fn test_redirect_allowed_host() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("http://example.com/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_blocked_host() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://evil.com/phishing", &config).is_err());
        assert!(validate_redirect_url("https://attacker.com", &config).is_err());
    }

    #[test]
    fn test_redirect_subdomain_when_allowed() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: true,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://www.example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://api.example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://sub.domain.example.com/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_subdomain_when_blocked() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://www.example.com/page", &config).is_err());
        assert!(validate_redirect_url("https://api.example.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_invalid_scheme() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("javascript:alert(1)", &config).is_err());
        assert!(
            validate_redirect_url("data:text/html,<script>alert(1)</script>", &config).is_err()
        );
        assert!(validate_redirect_url("ftp://example.com/file", &config).is_err());
        assert!(validate_redirect_url("file:///etc/passwd", &config).is_err());
    }

    #[test]
    fn test_redirect_multiple_allowed_hosts() {
        let config = RedirectConfig {
            allowed_hosts: vec![
                "example.com".to_string(),
                "trusted.com".to_string(),
                "localhost".to_string(),
            ],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://trusted.com/page", &config).is_ok());
        assert!(validate_redirect_url("http://localhost:3000/page", &config).is_ok());
        assert!(validate_redirect_url("https://evil.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_invalid_url_format() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("not a url", &config).is_err());
        assert!(validate_redirect_url("ht!tp://example.com", &config).is_err());
    }

    #[test]
    fn test_redirect_with_port() {
        let config = RedirectConfig {
            allowed_hosts: vec!["localhost".to_string(), "example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("http://localhost:3000/page", &config).is_ok());
        assert!(validate_redirect_url("https://example.com:8443/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_with_query_and_fragment() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page?foo=bar", &config).is_ok());
        assert!(validate_redirect_url("https://example.com/page#section", &config).is_ok());
        assert!(validate_redirect_url("https://example.com/page?foo=bar#section", &config).is_ok());
        assert!(validate_redirect_url("/page?foo=bar#section", &config).is_ok());
    }

    #[test]
    fn test_redirect_empty_allowed_hosts() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("/page", &config).is_ok());

        assert!(validate_redirect_url("https://example.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_case_sensitivity() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://EXAMPLE.COM/page", &config).is_ok());
        assert!(validate_redirect_url("https://Example.Com/page", &config).is_ok());

        assert!(validate_redirect_url("https://evil.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_homograph_attack_prevention() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://examp1e.com/page", &config).is_err());
        assert!(validate_redirect_url("https://example.co/page", &config).is_err());
        assert!(validate_redirect_url("https://examplecom.com/page", &config).is_err());
    }

    #[test]
    fn test_is_reserved_export_name_then() {
        use crate::server::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(is_reserved_export_name("catch"));
        assert!(is_reserved_export_name("finally"));
    }

    #[test]
    fn test_is_reserved_export_name_object_methods() {
        use crate::server::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("toString"));
        assert!(is_reserved_export_name("valueOf"));
        assert!(is_reserved_export_name("toLocaleString"));
        assert!(is_reserved_export_name("constructor"));
    }

    #[test]
    fn test_is_reserved_export_name_symbols() {
        use crate::server::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("Symbol"));
        assert!(is_reserved_export_name("@@iterator"));
        assert!(is_reserved_export_name("@@toStringTag"));
    }

    #[test]
    fn test_is_reserved_export_name_allows_valid_names() {
        use crate::server::actions::is_reserved_export_name;

        assert!(!is_reserved_export_name("getData"));
        assert!(!is_reserved_export_name("submitForm"));
        assert!(!is_reserved_export_name("updateUser"));
        assert!(!is_reserved_export_name("deleteItem"));
        assert!(!is_reserved_export_name("GET"));
        assert!(!is_reserved_export_name("POST"));
        assert!(!is_reserved_export_name("myAction"));
    }

    #[test]
    fn test_is_reserved_export_name_case_sensitive() {
        use crate::server::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(!is_reserved_export_name("Then"));
        assert!(!is_reserved_export_name("THEN"));

        assert!(is_reserved_export_name("catch"));
        assert!(!is_reserved_export_name("Catch"));
    }

    #[test]
    fn test_is_reserved_export_name_similar_names() {
        use crate::server::actions::is_reserved_export_name;

        assert!(!is_reserved_export_name("thenDo"));
        assert!(!is_reserved_export_name("catchError"));
        assert!(!is_reserved_export_name("finallyDone"));
        assert!(!is_reserved_export_name("myThen"));
    }

    #[test]
    fn test_cve_2025_55182_wide_array_dos_attack() {
        let config = ValidationConfig {
            max_depth: 10,
            max_total_elements: 10_000,
            max_array_length: 1_000,
            ..Default::default()
        };

        let mut outer_array = Vec::new();
        for _ in 0..20 {
            outer_array.push(json!(vec![1; 600]));
        }
        let wide_nested = json!({ "data": outer_array });

        let result = validate_and_sanitize_args(&[wide_nested], &config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Maximum array nesting exceeded") || err_msg.contains("12000 > 10000"),
            "Expected array nesting error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_cve_2025_55182_string_accumulation_dos() {
        let config = ValidationConfig {
            max_depth: 10,
            max_total_elements: 50_000,
            max_string_length: 10_000,
            ..Default::default()
        };

        let strings: Vec<_> = (0..10).map(|_| json!("A".repeat(6_000))).collect();
        let many_strings = json!({ "strings": strings });

        let result = validate_and_sanitize_args(&[many_strings], &config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Maximum array nesting exceeded"),
            "Expected cumulative limit error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_cve_2025_55182_fork_detection() {
        let config = ValidationConfig {
            max_depth: 5,
            max_total_elements: 1_000,
            max_array_length: 500,
            ..Default::default()
        };

        let single_child = json!({ "data": [vec![1; 500]] });
        assert!(validate_and_sanitize_args(&[single_child], &config).is_ok());

        let forked = json!({ "data": [vec![1; 500], vec![2; 500]] });
        let result = validate_and_sanitize_args(&[forked], &config);

        assert!(result.is_err(), "Expected fork with >1000 elements to fail");
    }

    #[test]
    fn test_cve_2025_55182_production_limits() {
        let prod_config = ValidationConfig::production();

        assert_eq!(prod_config.max_total_elements, 1_000_000);
        assert_eq!(prod_config.max_depth, 10);
        assert_eq!(prod_config.max_array_length, 1_000);
        assert_eq!(prod_config.max_string_length, 10_000);
    }

    #[test]
    fn test_cve_2025_55182_development_limits() {
        let dev_config = ValidationConfig::development();

        assert_eq!(dev_config.max_total_elements, 5_000_000);
        assert_eq!(dev_config.max_depth, 20);
        assert_eq!(dev_config.max_array_length, 5_000);
        assert_eq!(dev_config.max_string_length, 50_000);
    }
}
