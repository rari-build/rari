use super::super::layout::ComputedLayout;
use super::super::resources::fonts::FontContext;
use super::super::types::JsxChild;
use super::mask::MaskMemory;
use image::{Rgba, RgbaImage};
use parley::FontContext as ParleyFontContext;
use rustc_hash::FxHashMap;
use zeno::Scratch;

pub struct ImageRenderer {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) font_context: ParleyFontContext,
    pub(super) scratch: Scratch,
    pub(super) mask_memory: MaskMemory,
}

impl ImageRenderer {
    pub fn new(width: u32, height: u32, font_context: FontContext) -> Self {
        Self {
            width,
            height,
            font_context: font_context.inner,
            scratch: Scratch::new(),
            mask_memory: MaskMemory::default(),
        }
    }

    pub fn render(&mut self, layout: &ComputedLayout) -> Result<RgbaImage, String> {
        let mut image = RgbaImage::new(self.width, self.height);

        for pixel in image.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }

        self.render_node(layout, &mut image)?;

        Ok(image)
    }

    fn render_node(
        &mut self,
        layout: &ComputedLayout,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        if let Some(bg) = layout.style.get("background").or(layout.style.get("backgroundColor")) {
            self.render_background(layout, bg, image, &mut self.mask_memory.clone())?;
        }

        match layout.element.element_type.as_str() {
            "img" => {
                self.render_image(layout, image)?;
            }
            _ => {
                if self.has_text_content(&layout.element) {
                    self.render_text(layout, image)?;
                }
            }
        }

        self.render_border(layout, image, &mut self.mask_memory.clone())?;

        for child in &layout.children {
            self.render_node(child, image)?;
        }

        Ok(())
    }

    pub(super) fn alpha_blend(&self, bg: Rgba<u8>, fg: Rgba<u8>) -> Rgba<u8> {
        let alpha = fg[3] as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        Rgba([
            ((fg[0] as f32 * alpha + bg[0] as f32 * inv_alpha) as u8),
            ((fg[1] as f32 * alpha + bg[1] as f32 * inv_alpha) as u8),
            ((fg[2] as f32 * alpha + bg[2] as f32 * inv_alpha) as u8),
            255,
        ])
    }

    fn has_text_content(&self, element: &super::super::types::JsxElement) -> bool {
        element.children.iter().any(|child| matches!(child, JsxChild::Text(_)))
    }

    pub(super) fn parse_font_weight(&self, style: &FxHashMap<String, String>) -> u16 {
        style
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
            .unwrap_or(400)
    }

    pub(super) fn parse_color(&self, color_str: &str) -> Rgba<u8> {
        match color_str {
            "black" => Rgba([0, 0, 0, 255]),
            "white" => Rgba([255, 255, 255, 255]),
            "red" => Rgba([255, 0, 0, 255]),
            "green" => Rgba([0, 255, 0, 255]),
            "blue" => Rgba([0, 0, 255, 255]),
            _ if color_str.starts_with('#') => {
                let hex = color_str.trim_start_matches('#');
                if hex.len() == 6 {
                    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                    Rgba([r, g, b, 255])
                } else {
                    Rgba([0, 0, 0, 255])
                }
            }
            _ if color_str.starts_with("rgb(") => {
                let inner = color_str.trim_start_matches("rgb(").trim_end_matches(')');
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if parts.len() == 3 {
                    let r = parts[0].parse().unwrap_or(0);
                    let g = parts[1].parse().unwrap_or(0);
                    let b = parts[2].parse().unwrap_or(0);
                    Rgba([r, g, b, 255])
                } else {
                    Rgba([0, 0, 0, 255])
                }
            }
            _ => Rgba([0, 0, 0, 255]),
        }
    }
}
