#[cfg(test)]
mod tests {
    use crate::server::actions::{is_dangerous_property, sanitize_args};
    use serde_json::json;

    #[test]
    fn test_sanitize_args_removes_proto() {
        let args = vec![json!({
            "__proto__": {
                "isAdmin": true
            },
            "username": "test"
        })];

        let sanitized = sanitize_args(&args);

        assert_eq!(sanitized.len(), 1);
        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("__proto__"));
        assert_eq!(obj.get("username").unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn test_sanitize_args_removes_constructor() {
        let args = vec![json!({
            "constructor": {
                "prototype": {
                    "isAdmin": true
                }
            },
            "data": "safe"
        })];

        let sanitized = sanitize_args(&args);

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("constructor"));
        assert_eq!(obj.get("data").unwrap().as_str().unwrap(), "safe");
    }

    #[test]
    fn test_sanitize_args_removes_prototype() {
        let args = vec![json!({
            "prototype": {
                "polluted": true
            },
            "normal": "value"
        })];

        let sanitized = sanitize_args(&args);

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("prototype"));
        assert_eq!(obj.get("normal").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_sanitize_args_nested_objects() {
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

        let sanitized = sanitize_args(&args);

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

        let sanitized = sanitize_args(&args);

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

        let sanitized = sanitize_args(&args);

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

        let sanitized = sanitize_args(&args);

        let obj = sanitized[0].as_object().unwrap();
        let level1 = obj.get("level1").unwrap().as_object().unwrap();
        let level2 = level1.get("level2").unwrap().as_object().unwrap();
        let level3 = level2.get("level3").unwrap().as_object().unwrap();
        assert!(!level3.contains_key("__proto__"));

        let level4 = level3.get("level4").unwrap().as_object().unwrap();
        assert!(!level4.contains_key("constructor"));
        assert_eq!(level4.get("safe").unwrap().as_str().unwrap(), "value");
    }
}
