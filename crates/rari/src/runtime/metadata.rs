use cow_utils::CowUtils;
use serde_json::{Map, Value};

pub fn merge_metadata(parent: &Value, child: &Value) -> Value {
    let parent_obj = parent.as_object();
    let child_obj = match child.as_object() {
        Some(obj) => obj,
        None => return parent.clone(),
    };

    let mut merged = parent_obj.cloned().unwrap_or_default();

    if let Some(child_title) = child_obj.get("title") {
        if let Some(child_title_str) = child_title.as_str() {
            if let Some(parent_title) = parent_obj.and_then(|p| p.get("title")) {
                if let Some(parent_title_obj) = parent_title.as_object() {
                    if let Some(template) =
                        parent_title_obj.get("template").and_then(|t| t.as_str())
                    {
                        let replaced = template.cow_replace("%s", child_title_str).into_owned();
                        merged.insert("title".to_string(), Value::String(replaced));
                    } else {
                        merged.insert("title".to_string(), child_title.clone());
                    }
                } else {
                    merged.insert("title".to_string(), child_title.clone());
                }
            } else {
                merged.insert("title".to_string(), child_title.clone());
            }
        } else {
            merged.insert("title".to_string(), child_title.clone());
        }
    }

    merge_simple_field(&mut merged, child_obj, "description");
    merge_simple_field(&mut merged, child_obj, "keywords");
    merge_simple_field(&mut merged, child_obj, "manifest");
    merge_simple_field(&mut merged, child_obj, "themeColor");
    merge_simple_field(&mut merged, child_obj, "viewport");
    merge_simple_field(&mut merged, child_obj, "canonical");

    merge_object_field(&mut merged, parent_obj, child_obj, "openGraph");
    merge_object_field(&mut merged, parent_obj, child_obj, "twitter");
    merge_object_field(&mut merged, parent_obj, child_obj, "robots");
    merge_object_field(&mut merged, parent_obj, child_obj, "icons");
    merge_object_field(&mut merged, parent_obj, child_obj, "appleWebApp");

    Value::Object(merged)
}

fn merge_simple_field(merged: &mut Map<String, Value>, child: &Map<String, Value>, field: &str) {
    if let Some(value) = child.get(field) {
        merged.insert(field.to_string(), value.clone());
    }
}

fn merge_object_field(
    merged: &mut Map<String, Value>,
    parent: Option<&Map<String, Value>>,
    child: &Map<String, Value>,
    field: &str,
) {
    if let Some(child_value) = child.get(field)
        && let Some(child_obj) = child_value.as_object()
    {
        let mut result = Map::new();

        if let Some(parent_value) = parent.and_then(|p| p.get(field))
            && let Some(parent_obj) = parent_value.as_object()
        {
            for (k, v) in parent_obj {
                result.insert(k.clone(), v.clone());
            }
        }

        for (k, v) in child_obj {
            result.insert(k.clone(), v.clone());
        }

        merged.insert(field.to_string(), Value::Object(result));
    }
}

pub fn finalize_metadata(metadata: &mut Value) {
    if let Some(obj) = metadata.as_object_mut()
        && let Some(title) = obj.get("title")
        && let Some(title_obj) = title.as_object()
    {
        if let Some(absolute) = title_obj.get("absolute") {
            obj.insert("title".to_string(), absolute.clone());
        } else if let Some(default) = title_obj.get("default") {
            obj.insert("title".to_string(), default.clone());
        } else if let Some(template) = title_obj.get("template") {
            obj.insert("title".to_string(), template.clone());
        } else {
            obj.insert("title".to_string(), Value::String(String::new()));
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_simple_fields() {
        let parent = json!({
            "title": "Parent Title",
            "description": "Parent Description"
        });
        let child = json!({
            "description": "Child Description",
            "keywords": ["test"]
        });

        let result = merge_metadata(&parent, &child);

        assert_eq!(result["title"], "Parent Title");
        assert_eq!(result["description"], "Child Description");
        assert_eq!(result["keywords"], json!(["test"]));
    }

    #[test]
    fn test_merge_title_with_template() {
        let parent = json!({
            "title": {
                "template": "%s | My Site"
            }
        });
        let child = json!({
            "title": "Page Title"
        });

        let result = merge_metadata(&parent, &child);

        assert_eq!(result["title"], "Page Title | My Site");
    }

    #[test]
    fn test_merge_object_fields() {
        let parent = json!({
            "openGraph": {
                "siteName": "My Site",
                "type": "website"
            }
        });
        let child = json!({
            "openGraph": {
                "title": "Page Title",
                "type": "article"
            }
        });

        let result = merge_metadata(&parent, &child);

        let og = result["openGraph"].as_object().unwrap();
        assert_eq!(og["siteName"], "My Site");
        assert_eq!(og["title"], "Page Title");
        assert_eq!(og["type"], "article");
    }

    #[test]
    fn test_finalize_metadata_absolute() {
        let mut metadata = json!({
            "title": {
                "absolute": "Absolute Title",
                "default": "Default Title"
            }
        });

        finalize_metadata(&mut metadata);

        assert_eq!(metadata["title"], "Absolute Title");
    }

    #[test]
    fn test_finalize_metadata_default() {
        let mut metadata = json!({
            "title": {
                "default": "Default Title"
            }
        });

        finalize_metadata(&mut metadata);

        assert_eq!(metadata["title"], "Default Title");
    }

    #[test]
    fn test_finalize_metadata_string() {
        let mut metadata = json!({
            "title": "String Title"
        });

        finalize_metadata(&mut metadata);

        assert_eq!(metadata["title"], "String Title");
    }

    #[test]
    fn test_finalize_metadata_template_only() {
        let mut metadata = json!({
            "title": {
                "template": "%s | My Site"
            }
        });

        finalize_metadata(&mut metadata);

        assert_eq!(metadata["title"], "%s | My Site");
    }

    #[test]
    fn test_finalize_metadata_empty_object() {
        let mut metadata = json!({
            "title": {}
        });

        finalize_metadata(&mut metadata);

        assert_eq!(metadata["title"], "");
    }
}
