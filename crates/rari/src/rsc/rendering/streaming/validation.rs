use rustc_hash::FxHashSet;

pub fn validate_suspense_boundaries(rsc_data: &serde_json::Value) -> Result<(), String> {
    let mut fallback_refs = FxHashSet::default();
    let mut duplicate_fallbacks = Vec::new();

    fn check_for_duplicates(
        value: &serde_json::Value,
        fallback_refs: &mut FxHashSet<String>,
        duplicates: &mut Vec<String>,
    ) {
        if let Some(arr) = value.as_array() {
            if arr.len() >= 4
                && arr[0].as_str() == Some("$")
                && arr[1].as_str() == Some("react.suspense")
                && let Some(props) = arr[3].as_object()
                && let Some(fallback) = props.get("fallback")
            {
                let fallback_str = serde_json::to_string(fallback).unwrap_or_default();

                if !fallback_refs.insert(fallback_str.clone()) {
                    let boundary_id = props
                        .get("~boundaryId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    tracing::warn!(
                        "Duplicate fallback content detected for boundary '{}'",
                        boundary_id
                    );

                    duplicates.push(boundary_id);
                }
            }

            for item in arr {
                check_for_duplicates(item, fallback_refs, duplicates);
            }
        } else if let Some(obj) = value.as_object() {
            for (_, v) in obj {
                check_for_duplicates(v, fallback_refs, duplicates);
            }
        }
    }

    check_for_duplicates(rsc_data, &mut fallback_refs, &mut duplicate_fallbacks);

    if !duplicate_fallbacks.is_empty() {
        let error_msg = format!(
            "Duplicate fallback content detected for boundaries: {:?}",
            duplicate_fallbacks
        );
        tracing::error!("{}", error_msg);
        return Err(error_msg);
    }

    Ok(())
}
