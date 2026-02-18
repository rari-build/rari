use super::super::layout::ComputedLayout;
use super::super::types::{JsxChild, JsxElement};
use super::renderer::ImageRenderer;
use cow_utils::CowUtils;
use image::{Rgba, RgbaImage};

const SVG_ELEMENTS: &[&str] = &[
    "svg",
    "g",
    "path",
    "circle",
    "ellipse",
    "rect",
    "line",
    "polyline",
    "polygon",
    "defs",
    "use",
    "symbol",
    "clipPath",
    "mask",
    "linearGradient",
    "radialGradient",
    "stop",
    "pattern",
    "image",
    "text",
    "tspan",
    "textPath",
    "switch",
    "foreignObject",
];

pub fn is_svg_element(element_type: &str) -> bool {
    SVG_ELEMENTS.contains(&element_type)
}

impl ImageRenderer {
    pub(super) fn render_svg(
        &mut self,
        layout: &ComputedLayout,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        let width = layout.width as u32;
        let height = layout.height as u32;

        if width == 0 || height == 0 {
            return Ok(());
        }

        let svg_string = jsx_to_svg_string(&layout.element);

        let options = resvg::usvg::Options {
            default_size: resvg::usvg::Size::from_wh(width as f32, height as f32).unwrap_or_else(
                || {
                    resvg::usvg::Size::from_wh(100.0, 100.0)
                        .expect("hardcoded fallback size is always valid")
                },
            ),
            ..Default::default()
        };

        let tree = resvg::usvg::Tree::from_str(&svg_string, &options)
            .map_err(|e| format!("SVG parse error: {e}"))?;

        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("Failed to create pixmap")?;

        let svg_size = tree.size();
        let scale_x = width as f32 / svg_size.width();
        let scale_y = height as f32 / svg_size.height();
        let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let x_start = layout.x as u32;
        let y_start = layout.y as u32;
        let data = pixmap.data();

        for py in 0..height {
            for px in 0..width {
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 >= data.len() {
                    continue;
                }
                let a = data[idx + 3];
                if a == 0 {
                    continue;
                }

                let (r, g, b) = if a == 255 {
                    (data[idx], data[idx + 1], data[idx + 2])
                } else {
                    let af = a as f32 / 255.0;
                    (
                        (data[idx] as f32 / af).min(255.0) as u8,
                        (data[idx + 1] as f32 / af).min(255.0) as u8,
                        (data[idx + 2] as f32 / af).min(255.0) as u8,
                    )
                };

                let canvas_x = x_start + px;
                let canvas_y = y_start + py;
                if canvas_x < self.width && canvas_y < self.height {
                    let bg = image.get_pixel(canvas_x, canvas_y);
                    let fg = Rgba([r, g, b, a]);
                    let blended = self.alpha_blend(*bg, fg);
                    image.put_pixel(canvas_x, canvas_y, blended);
                }
            }
        }

        Ok(())
    }
}

fn jsx_to_svg_string(element: &JsxElement) -> String {
    let mut buf = String::with_capacity(512);
    write_element(element, &mut buf);
    buf
}

fn write_element(element: &JsxElement, buf: &mut String) {
    let tag = &element.element_type;
    buf.push('<');
    buf.push_str(tag);

    if let Some(obj) = element.props.as_object() {
        for (key, value) in obj {
            if matches!(key.as_str(), "children" | "key" | "ref") {
                continue;
            }
            let attr_name = camel_to_kebab(key);
            let attr_val = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            buf.push(' ');
            buf.push_str(&attr_name);
            buf.push_str("=\"");
            buf.push_str(&escape_xml(&attr_val));
            buf.push('"');
        }
    }

    if element.children.is_empty() {
        buf.push_str("/>");
    } else {
        buf.push('>');
        for child in &element.children {
            match child {
                JsxChild::Element(el) => write_element(el, buf),
                JsxChild::Text(t) => buf.push_str(&escape_xml(t)),
            }
        }
        buf.push_str("</");
        buf.push_str(tag);
        buf.push('>');
    }
}

fn camel_to_kebab(s: &str) -> String {
    match s {
        "viewBox" => "viewBox".to_string(),
        "gradientUnits" => "gradientUnits".to_string(),
        "gradientTransform" => "gradientTransform".to_string(),
        "patternUnits" => "patternUnits".to_string(),
        "patternTransform" => "patternTransform".to_string(),
        "patternContentUnits" => "patternContentUnits".to_string(),
        "clipPathUnits" => "clipPathUnits".to_string(),
        "maskUnits" => "maskUnits".to_string(),
        "maskContentUnits" => "maskContentUnits".to_string(),
        "spreadMethod" => "spreadMethod".to_string(),
        "markerUnits" => "markerUnits".to_string(),
        "markerWidth" => "markerWidth".to_string(),
        "markerHeight" => "markerHeight".to_string(),
        "refX" => "refX".to_string(),
        "refY" => "refY".to_string(),
        "startOffset" => "startOffset".to_string(),
        "textLength" => "textLength".to_string(),
        "lengthAdjust" => "lengthAdjust".to_string(),
        "numOctaves" => "numOctaves".to_string(),
        "baseFrequency" => "baseFrequency".to_string(),
        "stdDeviation" => "stdDeviation".to_string(),
        "tableValues" => "tableValues".to_string(),
        "filterUnits" => "filterUnits".to_string(),
        "primitiveUnits" => "primitiveUnits".to_string(),
        "xChannelSelector" => "xChannelSelector".to_string(),
        "yChannelSelector" => "yChannelSelector".to_string(),
        "edgeMode" => "edgeMode".to_string(),
        "stitchTiles" => "stitchTiles".to_string(),
        "preserveAspectRatio" => "preserveAspectRatio".to_string(),
        "xlinkHref" => "xlink:href".to_string(),
        "xmlSpace" => "xml:space".to_string(),
        "xmlLang" => "xml:lang".to_string(),

        "strokeWidth" => "stroke-width".to_string(),
        "strokeLinecap" => "stroke-linecap".to_string(),
        "strokeLinejoin" => "stroke-linejoin".to_string(),
        "strokeDasharray" => "stroke-dasharray".to_string(),
        "strokeDashoffset" => "stroke-dashoffset".to_string(),
        "strokeMiterlimit" => "stroke-miterlimit".to_string(),
        "strokeOpacity" => "stroke-opacity".to_string(),
        "fillOpacity" => "fill-opacity".to_string(),
        "fillRule" => "fill-rule".to_string(),
        "clipPath" => "clip-path".to_string(),
        "clipRule" => "clip-rule".to_string(),
        "colorInterpolation" => "color-interpolation".to_string(),
        "colorInterpolationFilters" => "color-interpolation-filters".to_string(),
        "dominantBaseline" => "dominant-baseline".to_string(),
        "enableBackground" => "enable-background".to_string(),
        "floodColor" => "flood-color".to_string(),
        "floodOpacity" => "flood-opacity".to_string(),
        "fontFamily" => "font-family".to_string(),
        "fontSize" => "font-size".to_string(),
        "fontSizeAdjust" => "font-size-adjust".to_string(),
        "fontStretch" => "font-stretch".to_string(),
        "fontStyle" => "font-style".to_string(),
        "fontVariant" => "font-variant".to_string(),
        "fontWeight" => "font-weight".to_string(),
        "imageRendering" => "image-rendering".to_string(),
        "letterSpacing" => "letter-spacing".to_string(),
        "lightingColor" => "lighting-color".to_string(),
        "markerEnd" => "marker-end".to_string(),
        "markerMid" => "marker-mid".to_string(),
        "markerStart" => "marker-start".to_string(),
        "shapeRendering" => "shape-rendering".to_string(),
        "stopColor" => "stop-color".to_string(),
        "stopOpacity" => "stop-opacity".to_string(),
        "textAnchor" => "text-anchor".to_string(),
        "textDecoration" => "text-decoration".to_string(),
        "textRendering" => "text-rendering".to_string(),
        "unicodeBidi" => "unicode-bidi".to_string(),
        "vectorEffect" => "vector-effect".to_string(),
        "wordSpacing" => "word-spacing".to_string(),
        "writingMode" => "writing-mode".to_string(),

        _ => s.to_string(),
    }
}

fn escape_xml(s: &str) -> String {
    s.cow_replace('&', "&amp;")
        .cow_replace('<', "&lt;")
        .cow_replace('>', "&gt;")
        .cow_replace('"', "&quot;")
        .into_owned()
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_is_svg_element_known_types() {
        for tag in &[
            "svg", "g", "path", "circle", "ellipse", "rect", "line", "polyline", "polygon", "defs",
            "use", "symbol", "text",
        ] {
            assert!(is_svg_element(tag), "{tag} should be an SVG element");
        }
    }

    #[test]
    fn test_is_svg_element_non_svg() {
        for tag in &["div", "span", "img", "p"] {
            assert!(!is_svg_element(tag), "{tag} should not be an SVG element");
        }
    }

    #[test]
    fn test_camel_to_kebab_presentation_attrs() {
        assert_eq!(camel_to_kebab("strokeWidth"), "stroke-width");
        assert_eq!(camel_to_kebab("fillOpacity"), "fill-opacity");
        assert_eq!(camel_to_kebab("stopColor"), "stop-color");
        assert_eq!(camel_to_kebab("fontFamily"), "font-family");
        assert_eq!(camel_to_kebab("strokeLinecap"), "stroke-linecap");
        assert_eq!(camel_to_kebab("strokeDasharray"), "stroke-dasharray");
    }

    #[test]
    fn test_camel_to_kebab_preserves_mixed_case() {
        assert_eq!(camel_to_kebab("viewBox"), "viewBox");
        assert_eq!(camel_to_kebab("gradientUnits"), "gradientUnits");
        assert_eq!(camel_to_kebab("preserveAspectRatio"), "preserveAspectRatio");
        assert_eq!(camel_to_kebab("spreadMethod"), "spreadMethod");
    }

    #[test]
    fn test_camel_to_kebab_passthrough() {
        assert_eq!(camel_to_kebab("fill"), "fill");
        assert_eq!(camel_to_kebab("stroke"), "stroke");
        assert_eq!(camel_to_kebab("opacity"), "opacity");
        assert_eq!(camel_to_kebab("transform"), "transform");
        assert_eq!(camel_to_kebab("d"), "d");
        assert_eq!(camel_to_kebab("cx"), "cx");
        assert_eq!(camel_to_kebab("r"), "r");
        assert_eq!(camel_to_kebab("x"), "x");
        assert_eq!(camel_to_kebab("y"), "y");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(escape_xml("normal"), "normal");
    }

    #[test]
    fn test_jsx_to_svg_simple_path() {
        let element = JsxElement {
            element_type: "svg".to_string(),
            props: serde_json::json!({
                "xmlns": "http://www.w3.org/2000/svg",
                "viewBox": "0 0 100 100",
                "width": "100",
                "height": "100"
            }),
            children: vec![JsxChild::Element(Box::new(JsxElement {
                element_type: "path".to_string(),
                props: serde_json::json!({ "d": "M0 0 L100 100", "fill": "#fff" }),
                children: vec![],
            }))],
        };
        let svg = jsx_to_svg_string(&element);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("viewBox=\"0 0 100 100\""));
        assert!(svg.contains("<path"));
        assert!(svg.contains("fill=\"#fff\""));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_jsx_to_svg_self_closing_leaf() {
        let element = JsxElement {
            element_type: "circle".to_string(),
            props: serde_json::json!({ "cx": "50", "cy": "50", "r": "25" }),
            children: vec![],
        };
        let mut buf = String::new();
        write_element(&element, &mut buf);
        assert!(buf.ends_with("/>"));
        assert!(!buf.contains("</circle>"));
    }

    #[test]
    fn test_jsx_to_svg_nested_g_inherits_fill() {
        let element = JsxElement {
            element_type: "svg".to_string(),
            props: serde_json::json!({ "xmlns": "http://www.w3.org/2000/svg", "viewBox": "0 0 10 10" }),
            children: vec![JsxChild::Element(Box::new(JsxElement {
                element_type: "g".to_string(),
                props: serde_json::json!({ "fill": "#ff0000" }),
                children: vec![JsxChild::Element(Box::new(JsxElement {
                    element_type: "rect".to_string(),
                    props: serde_json::json!({ "x": "0", "y": "0", "width": "10", "height": "10" }),
                    children: vec![],
                }))],
            }))],
        };
        let svg = jsx_to_svg_string(&element);
        assert!(svg.contains("fill=\"#ff0000\""));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn test_jsx_to_svg_rari_logo_parses_with_resvg() {
        let element = JsxElement {
            element_type: "svg".to_string(),
            props: serde_json::json!({
                "xmlns": "http://www.w3.org/2000/svg",
                "width": "437",
                "height": "145",
                "fill": "none",
                "viewBox": "0 0 437 145"
            }),
            children: vec![JsxChild::Element(Box::new(JsxElement {
                element_type: "g".to_string(),
                props: serde_json::json!({}),
                children: vec![JsxChild::Element(Box::new(JsxElement {
                    element_type: "g".to_string(),
                    props: serde_json::json!({ "fill": "#fff" }),
                    children: vec![
                        JsxChild::Element(Box::new(JsxElement {
                            element_type: "path".to_string(),
                            props: serde_json::json!({ "d": "m436.808 0-5.6 24.6h-46.2l5.6-24.6zm-8.2 35.2-24.4 106h-46.2l24.6-106z" }),
                            children: vec![],
                        })),
                        JsxChild::Element(Box::new(JsxElement {
                            element_type: "path".to_string(),
                            props: serde_json::json!({ "d": "M253.303 64.8q0 7.4-2.6 18.6l-9.2 40q-1 3.6-1 7.6 0 1.4 2.2 10.2h-49.4q-.6-5.6.2-11.8-10.8 5.8-12.2 6.6-17.8 8.4-39.8 8.4-8.6 0-15.4-1.6a45 45 0 0 1-6.8-2.2q-3.4-1.4-8.8-4.6-5.2-3.4-8.6-9.2-3.2-6-3.2-13.8 0-5 1.6-9.8 4.6-13 16.8-19.2 2.4-1.2 5.4-2.2 3-1.2 6.6-1.8 3.8-.8 6.8-1.4 3.2-.6 7.8-1 4.6-.6 7.2-.8 2.8-.4 7.8-.6t6.8-.2q2-.2 6.8-.4 5-.2 5.8-.2 4.4-.2 13.4-.4 9-.4 13.4-.6.8-5.8-1.4-9.2-4-6.6-20.2-6.6-14.8 0-20.2 3.8-1.6 1.2-4.2 5.4h-45.6q4.6-12.4 8.2-16.8 5.2-6.4 13.6-10.6t19-5.8 18.2-2q7.6-.6 17.8-.6 37.6 0 51.2 8.6 12 7.6 12 24.2m-53 29.4q-16.8-1-32.2 1.4-11 1.6-15.2 4-5.2 2.6-5.2 7.6 0 2.2 1.8 4.8 3.8 4.6 13 4.6 23.8 0 34-14.4 2-2.6 3.8-8" }),
                            children: vec![],
                        })),
                        JsxChild::Element(Box::new(JsxElement {
                            element_type: "path".to_string(),
                            props: serde_json::json!({ "d": "m108.4 33-9.6 41.6q-7.8-2.2-15.6-2.2-12.8 0-19.2 6.4-2.6 2.6-4.8 8.2-3 8.4-13 54.2H0l24.4-106h46l-3.8 16.6q4.8-5.6 7.6-8 12.4-11.4 27-11.4 3.6 0 7.2.6" }),
                            children: vec![],
                        })),
                    ],
                }))],
            }))],
        };

        let svg = jsx_to_svg_string(&element);
        assert!(svg.contains("viewBox=\"0 0 437 145\""));
        assert!(svg.contains("fill=\"#fff\""));

        let options = resvg::usvg::Options::default();
        let result = resvg::usvg::Tree::from_str(&svg, &options);
        assert!(result.is_ok(), "usvg failed to parse Rari logo: {:?}", result.err());
    }

    #[test]
    fn test_jsx_to_svg_gradient() {
        let element = JsxElement {
            element_type: "svg".to_string(),
            props: serde_json::json!({ "xmlns": "http://www.w3.org/2000/svg", "viewBox": "0 0 100 100" }),
            children: vec![
                JsxChild::Element(Box::new(JsxElement {
                    element_type: "defs".to_string(),
                    props: serde_json::json!({}),
                    children: vec![JsxChild::Element(Box::new(JsxElement {
                        element_type: "linearGradient".to_string(),
                        props: serde_json::json!({ "id": "grad1", "gradientUnits": "userSpaceOnUse" }),
                        children: vec![
                            JsxChild::Element(Box::new(JsxElement {
                                element_type: "stop".to_string(),
                                props: serde_json::json!({ "offset": "0", "stopColor": "#ff0000" }),
                                children: vec![],
                            })),
                            JsxChild::Element(Box::new(JsxElement {
                                element_type: "stop".to_string(),
                                props: serde_json::json!({ "offset": "1", "stopColor": "#0000ff" }),
                                children: vec![],
                            })),
                        ],
                    }))],
                })),
                JsxChild::Element(Box::new(JsxElement {
                    element_type: "rect".to_string(),
                    props: serde_json::json!({ "x": "0", "y": "0", "width": "100", "height": "100", "fill": "url(#grad1)" }),
                    children: vec![],
                })),
            ],
        };
        let svg = jsx_to_svg_string(&element);
        assert!(svg.contains("linearGradient"));
        assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
        assert!(svg.contains("stop-color=\"#ff0000\""));

        let options = resvg::usvg::Options::default();
        let result = resvg::usvg::Tree::from_str(&svg, &options);
        assert!(result.is_ok(), "usvg failed to parse gradient SVG: {:?}", result.err());
    }
}
