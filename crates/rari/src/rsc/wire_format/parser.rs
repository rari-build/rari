use crate::error::RariError;
use crate::rsc::types::{RscElement, SuspenseBoundary};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct PromiseRef {
    pub promise_id: String,
    pub boundary_id: String,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct StreamingState {
    pub initial_html: String,
    pub boundaries: Vec<SuspenseBoundary>,
    pub promises: FxHashMap<String, PromiseRef>,
    pub resolved: FxHashSet<String>,
}

pub struct RscWireFormatParser {
    lines: Vec<String>,
    elements: FxHashMap<u32, RscElement>,
}

impl RscWireFormatParser {
    pub fn new(rsc_output: &str) -> Self {
        Self {
            lines: rsc_output.lines().map(|s| s.to_string()).collect(),
            elements: FxHashMap::default(),
        }
    }

    pub fn parse(&mut self) -> Result<(), RariError> {
        for line in &self.lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (row_id, element) = self.parse_line(line)?;
            self.elements.insert(row_id, element);
        }

        debug!("Parsed {} RSC elements from {} lines", self.elements.len(), self.lines.len());

        Ok(())
    }

    fn parse_line(&self, line: &str) -> Result<(u32, RscElement), RariError> {
        let colon_pos = line.find(':').ok_or_else(|| {
            RariError::internal(format!("Invalid RSC line format: missing colon in '{}'", line))
        })?;

        let (id_str, data_str) = line.split_at(colon_pos);
        let data_str = &data_str[1..];

        let row_id = id_str
            .parse::<u32>()
            .map_err(|e| RariError::internal(format!("Invalid row ID '{}': {}", id_str, e)))?;

        if data_str.starts_with('I') {
            return Ok((row_id, RscElement::Text(String::new())));
        }

        let json_value: JsonValue = serde_json::from_str(data_str)
            .map_err(|e| RariError::internal(format!("Invalid JSON in RSC line: {}", e)))?;

        let element = self.parse_json_element(&json_value)?;

        Ok((row_id, element))
    }

    fn parse_json_element(&self, value: &JsonValue) -> Result<RscElement, RariError> {
        match value {
            JsonValue::String(s) => {
                if s.starts_with('$') {
                    Ok(RscElement::Reference(s.clone()))
                } else {
                    Ok(RscElement::Text(s.clone()))
                }
            }

            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    return Err(RariError::internal("Empty array in RSC element".to_string()));
                }

                if let Some(JsonValue::String(marker)) = arr.first()
                    && marker == "$"
                {
                    return self.parse_react_element(arr);
                }

                Ok(RscElement::Text(serde_json::to_string(value).unwrap_or_default()))
            }

            JsonValue::Number(n) => Ok(RscElement::Text(n.to_string())),
            JsonValue::Bool(b) => Ok(RscElement::Text(b.to_string())),
            JsonValue::Null => Ok(RscElement::Text(String::new())),

            JsonValue::Object(_) => {
                Ok(RscElement::Text(serde_json::to_string(value).unwrap_or_default()))
            }
        }
    }

    fn parse_react_element(&self, arr: &[JsonValue]) -> Result<RscElement, RariError> {
        if arr.len() < 4 {
            return Err(RariError::internal(format!(
                "Invalid React element: expected 4 elements, got {}",
                arr.len()
            )));
        }

        let tag = arr[1]
            .as_str()
            .ok_or_else(|| RariError::internal("React element tag must be a string".to_string()))?
            .to_string();

        let key = arr[2].as_str().map(|s| s.to_string());

        let props_value = &arr[3];
        let props = if let JsonValue::Object(obj) = props_value {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            FxHashMap::default()
        };

        if tag == "Suspense" || tag == "react.suspense" {
            return self.parse_suspense_element(key, props);
        }

        if tag == "Promise" || tag == "react.promise" {
            return self.parse_promise_element(props);
        }

        Ok(RscElement::Component { tag, key, props })
    }

    fn parse_suspense_element(
        &self,
        key: Option<String>,
        props: FxHashMap<String, JsonValue>,
    ) -> Result<RscElement, RariError> {
        let fallback_ref = props.get("fallback").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let children_ref = props.get("children").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let boundary_id = props
            .get("__boundary_id")
            .or_else(|| props.get("boundaryId"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| key.clone())
            .unwrap_or_else(|| format!("boundary_{}", uuid::Uuid::new_v4()));

        debug!(
            "Parsed Suspense boundary: id={}, fallback={}, children={}",
            boundary_id, fallback_ref, children_ref
        );

        Ok(RscElement::Suspense { fallback_ref, children_ref, boundary_id, props })
    }

    fn parse_promise_element(
        &self,
        props: FxHashMap<String, JsonValue>,
    ) -> Result<RscElement, RariError> {
        let promise_id = props
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("promise_{}", uuid::Uuid::new_v4()));

        debug!("Parsed Promise element: id={}", promise_id);

        Ok(RscElement::Promise { promise_id })
    }

    pub fn find_suspense_boundaries(&self) -> Vec<SuspenseBoundary> {
        let mut boundaries = Vec::new();

        for (row_id, element) in &self.elements {
            if let RscElement::Suspense { fallback_ref, children_ref, boundary_id, .. } = element {
                let boundary = SuspenseBoundary {
                    boundary_id: boundary_id.clone(),
                    fallback_ref: fallback_ref.clone(),
                    children_ref: children_ref.clone(),
                    has_promise: false,
                    promise_ids: Vec::new(),
                    row_id: *row_id,
                };

                boundaries.push(boundary);
            }
        }

        debug!("Found {} Suspense boundaries", boundaries.len());

        boundaries
    }

    pub fn find_promises(&self) -> Vec<PromiseRef> {
        let mut promises = Vec::new();

        for (row_id, element) in &self.elements {
            if let RscElement::Promise { promise_id } = element {
                let promise_ref = PromiseRef {
                    promise_id: promise_id.clone(),
                    boundary_id: String::new(),
                    element_ref: format!("$L{}", row_id),
                };

                promises.push(promise_ref);
            }
        }

        debug!("Found {} Promise elements", promises.len());

        promises
    }

    pub fn link_promises_to_boundaries(
        &self,
        mut boundaries: Vec<SuspenseBoundary>,
        mut promises: Vec<PromiseRef>,
    ) -> (Vec<SuspenseBoundary>, Vec<PromiseRef>) {
        for boundary in &mut boundaries {
            for promise in &mut promises {
                if boundary.children_ref == promise.element_ref {
                    promise.boundary_id = boundary.boundary_id.clone();
                    boundary.promise_ids.push(promise.promise_id.clone());
                    boundary.has_promise = true;

                    debug!(
                        "Linked promise {} to boundary {}",
                        promise.promise_id, boundary.boundary_id
                    );
                }
            }
        }

        (boundaries, promises)
    }

    pub fn elements(&self) -> &FxHashMap<u32, RscElement> {
        &self.elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_component() {
        let rsc = r#"0:["$","div",null,{"className":"container","children":"Hello"}]"#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let elements = parser.elements();
        assert_eq!(elements.len(), 1);

        if let Some(RscElement::Component { tag, props, .. }) = elements.get(&0) {
            assert_eq!(tag, "div");
            assert_eq!(props.get("className").and_then(|v| v.as_str()), Some("container"));
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_parse_suspense_boundary() {
        let rsc = r#"0:["$","Suspense",null,{"fallback":"$L1","children":"$L2","__boundary_id":"test-boundary"}]"#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let boundaries = parser.find_suspense_boundaries();
        assert_eq!(boundaries.len(), 1);

        let boundary = &boundaries[0];
        assert_eq!(boundary.boundary_id, "test-boundary");
        assert_eq!(boundary.fallback_ref, "$L1");
        assert_eq!(boundary.children_ref, "$L2");
    }

    #[test]
    fn test_parse_promise() {
        let rsc = r#"0:["$","Promise",null,{"id":"promise-1"}]"#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let promises = parser.find_promises();
        assert_eq!(promises.len(), 1);

        let promise = &promises[0];
        assert_eq!(promise.promise_id, "promise-1");
    }

    #[test]
    fn test_parse_reference() {
        let rsc = r#"0:"$L1""#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let elements = parser.elements();
        if let Some(RscElement::Reference(ref_str)) = elements.get(&0) {
            assert_eq!(ref_str, "$L1");
        } else {
            panic!("Expected Reference element");
        }
    }

    #[test]
    fn test_parse_text() {
        let rsc = r#"0:"Hello World""#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let elements = parser.elements();
        if let Some(RscElement::Text(text)) = elements.get(&0) {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected Text element");
        }
    }

    #[test]
    fn test_link_promises_to_boundaries() {
        let rsc = r#"0:["$","Suspense",null,{"fallback":"$L1","children":"$L2","__boundary_id":"boundary-1"}]
1:["$","div",null,{"children":"Loading..."}]
2:["$","Promise",null,{"id":"promise-1"}]"#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let boundaries = parser.find_suspense_boundaries();
        let promises = parser.find_promises();

        let (linked_boundaries, linked_promises) =
            parser.link_promises_to_boundaries(boundaries, promises);

        assert_eq!(linked_boundaries.len(), 1);
        assert_eq!(linked_promises.len(), 1);

        let boundary = &linked_boundaries[0];
        let promise = &linked_promises[0];

        assert_eq!(boundary.children_ref, "$L2");
        assert_eq!(promise.element_ref, "$L2");
        assert_eq!(promise.boundary_id, "boundary-1");
        assert!(boundary.has_promise);
        assert_eq!(boundary.promise_ids.len(), 1);
    }

    #[test]
    fn test_parse_multiple_elements() {
        let rsc = r#"0:["$","div",null,{"className":"root"}]
1:["$","Suspense",null,{"fallback":"$L2","children":"$L3"}]
2:["$","div",null,{"children":"Loading..."}]
3:["$","Promise",null,{"id":"async-data"}]"#;

        let mut parser = RscWireFormatParser::new(rsc);
        assert!(parser.parse().is_ok());

        let elements = parser.elements();
        assert_eq!(elements.len(), 4);

        let boundaries = parser.find_suspense_boundaries();
        assert_eq!(boundaries.len(), 1);

        let promises = parser.find_promises();
        assert_eq!(promises.len(), 1);
    }
}
