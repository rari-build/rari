pub mod style;

use super::resources::fonts::FontContext;
use super::types::{JsxChild, JsxElement};
use parley::{FontContext as ParleyFontContext, LayoutContext, TextStyle};
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::cell::RefCell;
use taffy::prelude::*;

pub struct MeasureContext {
    font_context: RefCell<ParleyFontContext>,
}

pub struct LayoutEngine {
    taffy: TaffyTree<NodeData>,
    font_context: FontContext,
    measure_context: MeasureContext,
}

pub struct NodeData {
    pub element: JsxElement,
    pub style: FxHashMap<String, String>,
    pub has_text: bool,
}

impl LayoutEngine {
    pub fn new() -> Self {
        let font_context = FontContext::new();

        Self {
            taffy: TaffyTree::new(),
            measure_context: MeasureContext {
                font_context: RefCell::new(font_context.inner.clone()),
            },
            font_context,
        }
    }

    pub fn get_font_context(&self) -> FontContext {
        self.font_context.clone()
    }

    pub fn layout(
        &mut self,
        element: &JsxElement,
        width: f32,
        height: f32,
    ) -> Result<ComputedLayout, String> {
        self.taffy.clear();

        let mut inherited_color = None;
        let root_node = self.build_tree(element, &mut inherited_color)?;

        self.taffy
            .compute_layout_with_measure(
                root_node,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(height),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_node(
                        &self.measure_context,
                        known_dimensions,
                        available_space,
                        node_context,
                    )
                },
            )
            .map_err(|e| format!("Layout computation failed: {:?}", e))?;

        self.extract_layout(root_node, 0.0, 0.0)
    }

    fn build_tree(
        &mut self,
        element: &JsxElement,
        inherited_color: &mut Option<String>,
    ) -> Result<NodeId, String> {
        let mut style = self.parse_style(&element.props);

        if !style.contains_key("color") {
            if let Some(color) = inherited_color {
                style.insert("color".to_string(), color.clone());
            }
        } else if let Some(color) = style.get("color") {
            *inherited_color = Some(color.clone());
        }

        let has_text = self.has_text_content(element);
        let taffy_style = self.style_to_taffy(&style);

        let node_data = NodeData { element: element.clone(), style: style.clone(), has_text };

        let mut child_nodes = Vec::new();
        for child in &element.children {
            if let JsxChild::Element(child_element) = child {
                let child_node = self.build_tree(child_element, inherited_color)?;
                child_nodes.push(child_node);
            }
        }

        let node = if child_nodes.is_empty() {
            self.taffy
                .new_leaf_with_context(taffy_style, node_data)
                .map_err(|e| format!("Failed to create leaf node: {:?}", e))?
        } else {
            let node = self
                .taffy
                .new_with_children(taffy_style, &child_nodes)
                .map_err(|e| format!("Failed to create parent node: {:?}", e))?;

            self.taffy
                .set_node_context(node, Some(node_data))
                .map_err(|e| format!("Failed to set node context: {:?}", e))?;

            node
        };

        Ok(node)
    }

    fn has_text_content(&self, element: &JsxElement) -> bool {
        element.children.iter().any(|child| matches!(child, JsxChild::Text(_)))
    }

    fn parse_style(&self, props: &Value) -> FxHashMap<String, String> {
        let mut style_map = FxHashMap::default();

        if let Some(Value::Object(style_obj)) = props.get("style") {
            for (key, value) in style_obj {
                if let Value::String(v) = value {
                    style_map.insert(key.clone(), v.clone());
                } else if let Value::Number(n) = value {
                    style_map.insert(key.clone(), n.to_string());
                }
            }
        }

        style_map
    }

    fn style_to_taffy(&self, style: &FxHashMap<String, String>) -> Style {
        let mut taffy_style = Style::default();

        if let Some(display) = style.get("display") {
            taffy_style.display = match display.as_str() {
                "flex" => Display::Flex,
                "none" => Display::None,
                _ => Display::Block,
            };
        }

        if let Some(flex_direction) = style.get("flexDirection") {
            taffy_style.flex_direction = match flex_direction.as_str() {
                "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }

        taffy_style.align_items =
            style.get("alignItems").map(|align_items| match align_items.as_str() {
                "flex-start" | "start" => AlignItems::FlexStart,
                "flex-end" | "end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "baseline" => AlignItems::Baseline,
                "stretch" => AlignItems::Stretch,
                _ => AlignItems::Stretch,
            });

        if let Some(justify_content) = style.get("justifyContent") {
            taffy_style.justify_content = Some(match justify_content.as_str() {
                "flex-start" | "start" => JustifyContent::FlexStart,
                "flex-end" | "end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" => JustifyContent::SpaceAround,
                "space-evenly" => JustifyContent::SpaceEvenly,
                _ => JustifyContent::FlexStart,
            });
        }

        if let Some(flex) = style.get("flex")
            && let Ok(flex_val) = flex.parse::<f32>()
        {
            taffy_style.flex_grow = flex_val;
            taffy_style.flex_shrink = 1.0;
            taffy_style.flex_basis = Dimension::length(0.0);
        }

        if let Some(width) = style.get("width") {
            taffy_style.size.width = self.parse_dimension(width);
        }

        if let Some(height) = style.get("height") {
            taffy_style.size.height = self.parse_dimension(height);
        }

        if let Some(padding) = style.get("padding") {
            taffy_style.padding = self.parse_padding_margin(padding);
        }

        if let Some(padding_left) = style.get("paddingLeft") {
            taffy_style.padding.left = self.parse_length_percentage(padding_left);
        }
        if let Some(padding_right) = style.get("paddingRight") {
            taffy_style.padding.right = self.parse_length_percentage(padding_right);
        }
        if let Some(padding_top) = style.get("paddingTop") {
            taffy_style.padding.top = self.parse_length_percentage(padding_top);
        }
        if let Some(padding_bottom) = style.get("paddingBottom") {
            taffy_style.padding.bottom = self.parse_length_percentage(padding_bottom);
        }

        if let Some(margin) = style.get("margin") {
            taffy_style.margin = self.parse_padding_margin_auto(margin);
        }

        if let Some(margin_left) = style.get("marginLeft") {
            taffy_style.margin.left = self.parse_length_percentage_auto(margin_left);
        }
        if let Some(margin_right) = style.get("marginRight") {
            taffy_style.margin.right = self.parse_length_percentage_auto(margin_right);
        }
        if let Some(margin_top) = style.get("marginTop") {
            taffy_style.margin.top = self.parse_length_percentage_auto(margin_top);
        }
        if let Some(margin_bottom) = style.get("marginBottom") {
            taffy_style.margin.bottom = self.parse_length_percentage_auto(margin_bottom);
        }

        if let Some(gap) = style.get("gap") {
            let gap_value = self.parse_length_percentage(gap);
            taffy_style.gap = Size { width: gap_value, height: gap_value };
        }
        if let Some(row_gap) = style.get("rowGap") {
            taffy_style.gap.height = self.parse_length_percentage(row_gap);
        }
        if let Some(column_gap) = style.get("columnGap") {
            taffy_style.gap.width = self.parse_length_percentage(column_gap);
        }

        taffy_style
    }

    fn parse_dimension(&self, value: &str) -> Dimension {
        if value.ends_with('%') {
            if let Ok(percent) = value.trim_end_matches('%').parse::<f32>() {
                return Dimension::percent(percent / 100.0);
            }
        } else if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            return Dimension::length(px);
        }
        Dimension::auto()
    }

    fn parse_length_percentage(&self, value: &str) -> LengthPercentage {
        if value.ends_with('%') {
            if let Ok(percent) = value.trim_end_matches('%').parse::<f32>() {
                return LengthPercentage::percent(percent / 100.0);
            }
        } else if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            return LengthPercentage::length(px);
        }
        LengthPercentage::length(0.0)
    }

    fn parse_length_percentage_auto(&self, value: &str) -> LengthPercentageAuto {
        if value == "auto" {
            return LengthPercentageAuto::auto();
        }
        if value.ends_with('%') {
            if let Ok(percent) = value.trim_end_matches('%').parse::<f32>() {
                return LengthPercentageAuto::percent(percent / 100.0);
            }
        } else if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            return LengthPercentageAuto::length(px);
        }
        LengthPercentageAuto::length(0.0)
    }

    fn parse_padding_margin(&self, value: &str) -> Rect<LengthPercentage> {
        let parts: Vec<&str> = value.split_whitespace().collect();

        match parts.len() {
            1 => {
                let p = self.parse_length_percentage(parts[0]);
                Rect { left: p, right: p, top: p, bottom: p }
            }
            2 => {
                let vertical = self.parse_length_percentage(parts[0]);
                let horizontal = self.parse_length_percentage(parts[1]);
                Rect { left: horizontal, right: horizontal, top: vertical, bottom: vertical }
            }
            3 => {
                let top = self.parse_length_percentage(parts[0]);
                let horizontal = self.parse_length_percentage(parts[1]);
                let bottom = self.parse_length_percentage(parts[2]);
                Rect { left: horizontal, right: horizontal, top, bottom }
            }
            4 => {
                let top = self.parse_length_percentage(parts[0]);
                let right = self.parse_length_percentage(parts[1]);
                let bottom = self.parse_length_percentage(parts[2]);
                let left = self.parse_length_percentage(parts[3]);
                Rect { left, right, top, bottom }
            }
            _ => Rect {
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(0.0),
            },
        }
    }

    fn parse_padding_margin_auto(&self, value: &str) -> Rect<LengthPercentageAuto> {
        let parts: Vec<&str> = value.split_whitespace().collect();

        match parts.len() {
            1 => {
                let m = self.parse_length_percentage_auto(parts[0]);
                Rect { left: m, right: m, top: m, bottom: m }
            }
            2 => {
                let vertical = self.parse_length_percentage_auto(parts[0]);
                let horizontal = self.parse_length_percentage_auto(parts[1]);
                Rect { left: horizontal, right: horizontal, top: vertical, bottom: vertical }
            }
            3 => {
                let top = self.parse_length_percentage_auto(parts[0]);
                let horizontal = self.parse_length_percentage_auto(parts[1]);
                let bottom = self.parse_length_percentage_auto(parts[2]);
                Rect { left: horizontal, right: horizontal, top, bottom }
            }
            4 => {
                let top = self.parse_length_percentage_auto(parts[0]);
                let right = self.parse_length_percentage_auto(parts[1]);
                let bottom = self.parse_length_percentage_auto(parts[2]);
                let left = self.parse_length_percentage_auto(parts[3]);
                Rect { left, right, top, bottom }
            }
            _ => Rect {
                left: LengthPercentageAuto::length(0.0),
                right: LengthPercentageAuto::length(0.0),
                top: LengthPercentageAuto::length(0.0),
                bottom: LengthPercentageAuto::length(0.0),
            },
        }
    }

    fn extract_layout(
        &self,
        node: NodeId,
        parent_x: f32,
        parent_y: f32,
    ) -> Result<ComputedLayout, String> {
        let layout =
            self.taffy.layout(node).map_err(|e| format!("Failed to get layout: {:?}", e))?;
        let node_data = self.taffy.get_node_context(node).ok_or("No node data")?;

        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;

        let mut children = Vec::new();
        for child_id in
            self.taffy.children(node).map_err(|e| format!("Failed to get children: {:?}", e))?
        {
            children.push(self.extract_layout(child_id, x, y)?);
        }

        Ok(ComputedLayout {
            x,
            y,
            width: layout.size.width,
            height: layout.size.height,
            border: layout.border,
            padding: layout.padding,
            element: node_data.element.clone(),
            style: node_data.style.clone(),
            children,
        })
    }
}

fn measure_node(
    context: &MeasureContext,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    node_context: Option<&mut NodeData>,
) -> Size<f32> {
    let Some(node_data) = node_context else {
        return Size::ZERO;
    };

    if node_data.element.element_type == "img" {
        return measure_image(node_data, known_dimensions, available_space);
    }

    if !node_data.has_text {
        return Size::ZERO;
    }

    let text: String = node_data
        .element
        .children
        .iter()
        .filter_map(|child| match child {
            JsxChild::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    if text.is_empty() {
        return Size::ZERO;
    }

    let font_size =
        node_data.style.get("fontSize").and_then(|s| s.parse::<f32>().ok()).unwrap_or(16.0);

    let font_weight = node_data
        .style
        .get("fontWeight")
        .and_then(|w| match w.as_str() {
            "normal" => Some(400),
            "bold" => Some(700),
            "100" => Some(100),
            "200" => Some(200),
            "300" => Some(300),
            "400" => Some(400),
            "500" => Some(500),
            "600" => Some(600),
            "700" => Some(700),
            "800" => Some(800),
            "900" => Some(900),
            _ => w.parse::<u16>().ok(),
        })
        .unwrap_or(400);

    let max_width = known_dimensions.width.or(match available_space.width {
        AvailableSpace::Definite(w) => Some(w),
        AvailableSpace::MaxContent => None,
        AvailableSpace::MinContent => Some(0.0),
    });

    let (text_width, text_height) =
        measure_text_with_parley(&context.font_context, &text, font_size, font_weight, max_width);

    Size {
        width: known_dimensions.width.unwrap_or(text_width),
        height: known_dimensions.height.unwrap_or(text_height),
    }
}

fn measure_image(
    node_data: &NodeData,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        return Size { width, height };
    }

    let src = node_data.element.props.get("src").and_then(|v| v.as_str());
    let intrinsic_size = if let Some(src) = src {
        load_image_dimensions(src).unwrap_or(Size { width: 0.0, height: 0.0 })
    } else {
        Size { width: 0.0, height: 0.0 }
    };

    let width_prop =
        node_data.element.props.get("width").and_then(|v| v.as_f64()).map(|v| v as f32);

    let height_prop =
        node_data.element.props.get("height").and_then(|v| v.as_f64()).map(|v| v as f32);

    let width_is_100_percent = node_data.style.get("width").map(|w| w == "100%").unwrap_or(false);
    let height_is_100_percent = node_data.style.get("height").map(|h| h == "100%").unwrap_or(false);

    let final_width = known_dimensions
        .width
        .or({
            if width_is_100_percent {
                match available_space.width {
                    AvailableSpace::Definite(w) => Some(w),
                    _ => None,
                }
            } else {
                None
            }
        })
        .or(width_prop)
        .or(if intrinsic_size.width > 0.0 { Some(intrinsic_size.width) } else { None })
        .unwrap_or(0.0);

    let final_height = known_dimensions
        .height
        .or({
            if height_is_100_percent {
                match available_space.height {
                    AvailableSpace::Definite(h) => Some(h),
                    _ => None,
                }
            } else {
                None
            }
        })
        .or(height_prop)
        .or(if intrinsic_size.height > 0.0 { Some(intrinsic_size.height) } else { None })
        .unwrap_or(0.0);

    if final_width > 0.0
        && final_height == 0.0
        && intrinsic_size.height > 0.0
        && intrinsic_size.width > 0.0
    {
        let aspect_ratio = intrinsic_size.width / intrinsic_size.height;
        return Size { width: final_width, height: final_width / aspect_ratio };
    }

    if final_height > 0.0
        && final_width == 0.0
        && intrinsic_size.width > 0.0
        && intrinsic_size.height > 0.0
    {
        let aspect_ratio = intrinsic_size.width / intrinsic_size.height;
        return Size { width: final_height * aspect_ratio, height: final_height };
    }

    Size { width: final_width, height: final_height }
}

fn load_image_dimensions(src: &str) -> Option<Size<f32>> {
    use std::io::Read;

    if src.starts_with("http://") || src.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .ok()?;

        let response = client.get(src).send().ok()?;
        if !response.status().is_success() {
            return None;
        }

        const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
        let mut buffer = Vec::new();
        response.take(MAX_IMAGE_SIZE as u64).read_to_end(&mut buffer).ok()?;

        let img = image::load_from_memory(&buffer).ok()?;
        Some(Size { width: img.width() as f32, height: img.height() as f32 })
    } else if src.starts_with("data:") {
        use base64::{Engine as _, engine::general_purpose};
        let parts: Vec<&str> = src.splitn(2, ',').collect();
        if parts.len() != 2 {
            return None;
        }

        let header = parts[0];
        let data = parts[1];

        if header.contains("base64") {
            let decoded = general_purpose::STANDARD.decode(data).ok()?;
            let img = image::load_from_memory(&decoded).ok()?;
            Some(Size { width: img.width() as f32, height: img.height() as f32 })
        } else {
            None
        }
    } else {
        let img = image::open(src).ok()?;
        Some(Size { width: img.width() as f32, height: img.height() as f32 })
    }
}

fn measure_text_with_parley(
    font_context: &RefCell<ParleyFontContext>,
    text: &str,
    font_size: f32,
    font_weight: u16,
    max_width: Option<f32>,
) -> (f32, f32) {
    let root_style = TextStyle {
        font_size,
        font_weight: parley::style::FontWeight::new(font_weight as f32),
        ..Default::default()
    };

    let mut font_ctx = font_context.borrow_mut();
    let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();
    let mut builder = layout_cx.tree_builder(&mut font_ctx, 1.0, true, &root_style);

    builder.push_text(text);

    let (mut layout, _text) = builder.build();
    layout.break_all_lines(max_width);

    let (width, height) = layout.lines().fold((0.0, 0.0), |(max_w, total_h), line| {
        let metrics = line.metrics();
        (metrics.advance.max(max_w), total_h + metrics.line_height)
    });

    (width.ceil(), height.ceil())
}

#[derive(Debug, Clone)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub border: taffy::Rect<f32>,
    pub padding: taffy::Rect<f32>,
    pub element: JsxElement,
    pub style: FxHashMap<String, String>,
    pub children: Vec<ComputedLayout>,
}
