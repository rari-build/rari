use serde_json::Value;

pub fn escape_rsc_string(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('$') {
        let prefix = &value[0..2];
        let rest = &value[2..];

        if matches!(
            prefix,
            "$L" | "$@" | "$F" | "$T" | "$S" | "$W" | "$Q" | "$K" | "$Y" | "$i" | "$h"
        ) && (rest.is_empty() || rest.chars().next().is_some_and(|c| c.is_ascii_digit()))
        {
            return value.to_string();
        }

        return format!("${}", value);
    }

    value.to_string()
}

pub fn escape_rsc_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(escape_rsc_string(s)),
        Value::Object(map) => {
            let mut escaped = serde_json::Map::new();
            for (k, v) in map {
                escaped.insert(k.clone(), escape_rsc_value(v));
            }
            Value::Object(escaped)
        }
        Value::Array(arr) => {
            if arr.len() >= 4 && arr[0].as_str() == Some("$") && arr[1].is_string() {
                let mut result = Vec::with_capacity(arr.len());
                result.push(arr[0].clone());
                result.push(arr[1].clone());
                result.push(arr[2].clone());
                for item in &arr[3..] {
                    result.push(escape_rsc_value(item));
                }
                Value::Array(result)
            } else {
                Value::Array(arr.iter().map(escape_rsc_value).collect())
            }
        }
        _ => value.clone(),
    }
}

pub fn unescape_rsc_string(value: &str) -> String {
    if value.starts_with("$$") { value[1..].to_string() } else { value.to_string() }
}

pub fn unescape_rsc_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(unescape_rsc_string(s)),
        Value::Object(map) => {
            let mut unescaped = serde_json::Map::new();
            for (k, v) in map {
                unescaped.insert(k.clone(), unescape_rsc_value(v));
            }
            Value::Object(unescaped)
        }
        Value::Array(arr) => {
            if arr.len() >= 4 && arr[0].as_str() == Some("$") && arr[1].is_string() {
                let mut result = Vec::with_capacity(arr.len());
                result.push(arr[0].clone());
                result.push(arr[1].clone());
                result.push(arr[2].clone());
                for item in &arr[3..] {
                    result.push(unescape_rsc_value(item));
                }
                Value::Array(result)
            } else {
                Value::Array(arr.iter().map(unescape_rsc_value).collect())
            }
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_escape_rsc_string() {
        assert_eq!(escape_rsc_string("hello"), "hello");
        assert_eq!(escape_rsc_string("$L999"), "$L999");
        assert_eq!(escape_rsc_string("$@123"), "$@123");
        assert_eq!(escape_rsc_string("$F456"), "$F456");
        assert_eq!(escape_rsc_string("$T789"), "$T789");
        assert_eq!(escape_rsc_string("$already"), "$$already");
        assert_eq!(escape_rsc_string(""), "");
        assert_eq!(escape_rsc_string("no dollar"), "no dollar");
    }

    #[test]
    fn test_unescape_rsc_string() {
        assert_eq!(unescape_rsc_string("hello"), "hello");
        assert_eq!(unescape_rsc_string("$L999"), "$L999");
        assert_eq!(unescape_rsc_string("$@123"), "$@123");
        assert_eq!(unescape_rsc_string("$F456"), "$F456");
        assert_eq!(unescape_rsc_string("$T789"), "$T789");
        assert_eq!(unescape_rsc_string("$$triple"), "$triple");
        assert_eq!(unescape_rsc_string(""), "");
        assert_eq!(unescape_rsc_string("no dollar"), "no dollar");
        assert_eq!(unescape_rsc_string("$L999"), "$L999");
    }

    #[test]
    fn test_escape_unescape_roundtrip() {
        let test_cases = vec!["hello", "$L999", "$double", "$$triple", "", "no dollar"];

        for case in test_cases {
            let escaped = escape_rsc_string(case);
            let unescaped = unescape_rsc_string(&escaped);
            assert_eq!(unescaped, case, "Roundtrip failed for: {}", case);
        }
    }

    #[test]
    fn test_escape_preserves_rsc_references_in_objects() {
        let input = json!({
            "rscRef": "$L999",
            "normal": "hello",
            "customDollar": "$injection",
            "nested": {
                "value": "$@123"
            }
        });

        let escaped = escape_rsc_value(&input);

        assert_eq!(escaped["rscRef"], "$L999");
        assert_eq!(escaped["nested"]["value"], "$@123");
        assert_eq!(escaped["normal"], "hello");
        assert_eq!(escaped["customDollar"], "$$injection");
    }

    #[test]
    fn test_escape_preserves_rsc_references_in_arrays() {
        let input = json!(["$L999", "normal", "$@123", "$custom"]);

        let escaped = escape_rsc_value(&input);

        assert_eq!(escaped[0], "$L999");
        assert_eq!(escaped[2], "$@123");

        assert_eq!(escaped[1], "normal");
        assert_eq!(escaped[3], "$$custom");
    }

    #[test]
    fn test_escape_rsc_value_mixed() {
        let input = json!({
            "strings": ["$L1", "$@2", "normal"],
            "nested": {
                "deep": {
                    "value": "$F3"
                }
            },
            "number": 42,
            "bool": true,
            "null": null
        });

        let escaped = escape_rsc_value(&input);

        assert_eq!(escaped["strings"][0], "$L1");
        assert_eq!(escaped["strings"][1], "$@2");
        assert_eq!(escaped["strings"][2], "normal");
        assert_eq!(escaped["nested"]["deep"]["value"], "$F3");
        assert_eq!(escaped["number"], 42);
        assert_eq!(escaped["bool"], true);
        assert_eq!(escaped["null"], Value::Null);
    }

    #[test]
    fn test_unescape_rsc_value_object() {
        let input = json!({
            "message": "$$L999",
            "normal": "hello",
            "nested": {
                "value": "$$@123"
            }
        });

        let unescaped = unescape_rsc_value(&input);

        assert_eq!(unescaped["message"], "$L999");
        assert_eq!(unescaped["normal"], "hello");
        assert_eq!(unescaped["nested"]["value"], "$@123");
    }

    #[test]
    fn test_escape_unescape_value_roundtrip() {
        let input = json!({
            "rscRef": "$L999",
            "array": ["$@123", "normal", "$custom"],
            "nested": {
                "value": "$F456",
                "dollar": "$injection"
            }
        });

        let escaped = escape_rsc_value(&input);
        let unescaped = unescape_rsc_value(&escaped);

        assert_eq!(unescaped, input);
    }

    #[test]
    fn test_escape_preserves_rsc_element_structure() {
        let input = json!(["$", "div", null, {"className": "$L999", "data-custom": "$injection"}]);

        let escaped = escape_rsc_value(&input);

        assert_eq!(escaped[0], "$");
        assert_eq!(escaped[1], "div");
        assert_eq!(escaped[2], Value::Null);
        assert_eq!(escaped[3]["className"], "$L999");
        assert_eq!(escaped[3]["data-custom"], "$$injection");
    }

    #[test]
    fn test_escape_nested_rsc_elements() {
        let input = json!(["$", "div", null, {
            "className": "$injection",
            "children": ["$", "span", null, {"children": "$L123"}]
        }]);

        let escaped = escape_rsc_value(&input);

        assert_eq!(escaped[0], "$");
        assert_eq!(escaped[3]["className"], "$$injection");
        assert_eq!(escaped[3]["children"][0], "$");
        assert_eq!(escaped[3]["children"][3]["children"], "$L123");
    }
}
