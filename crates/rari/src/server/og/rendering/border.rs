use image::RgbaImage;
use zeno::Fill;

use super::{
    super::layout::ComputedLayout,
    mask::{MaskMemory, build_rounded_rect_path, mask_index},
    renderer::ImageRenderer,
};
use crate::utils::{cast, float};

#[derive(Debug, Clone, Copy)]
pub(super) struct BorderWidth {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub fn inset_by(&self, border: &BorderWidth) -> BorderRadius {
        BorderRadius {
            top_left: (self.top_left - border.top.max(border.left)).max(0.0),
            top_right: (self.top_right - border.top.max(border.right)).max(0.0),
            bottom_right: (self.bottom_right - border.bottom.max(border.right)).max(0.0),
            bottom_left: (self.bottom_left - border.bottom.max(border.left)).max(0.0),
        }
    }
}

impl ImageRenderer {
    pub(super) fn render_border(
        &self,
        layout: &ComputedLayout,
        image: &mut RgbaImage,
        mask_memory: &mut MaskMemory,
    ) -> Result<(), String> {
        let border_width = Self::parse_border_width(&layout.style);
        let border_color = Self::parse_border_color(&layout.style);

        if border_width.top == 0.0
            && border_width.right == 0.0
            && border_width.bottom == 0.0
            && border_width.left == 0.0
        {
            return Ok(());
        }

        let border_radius = Self::parse_border_radius(&layout.style);

        let x_start = cast::f32_to_u32(layout.x);
        let y_start = cast::f32_to_u32(layout.y);
        let width = layout.width;
        let height = layout.height;

        let has_radius = border_radius.top_left > 0.0
            || border_radius.top_right > 0.0
            || border_radius.bottom_right > 0.0
            || border_radius.bottom_left > 0.0;

        if has_radius {
            self.draw_rounded_border_masked(
                image,
                mask_memory,
                x_start,
                y_start,
                width,
                height,
                border_width,
                border_color,
                border_radius,
            );
        } else {
            self.draw_rect_border(
                image,
                x_start,
                y_start,
                width,
                height,
                border_width,
                border_color,
            );
        }

        Ok(())
    }

    #[expect(clippy::too_many_arguments)]
    fn draw_rect_border(
        &self,
        image: &mut RgbaImage,
        x: u32,
        y: u32,
        width: f32,
        height: f32,
        border_width: BorderWidth,
        color: image::Rgba<u8>,
    ) {
        let x_end =
            cast::f32_to_u32((float::u32_to_f32(x) + width).min(float::u32_to_f32(self.width)));
        let y_end =
            cast::f32_to_u32((float::u32_to_f32(y) + height).min(float::u32_to_f32(self.height)));

        if border_width.top > 0.0 {
            let top_height = cast::f32_to_u32(border_width.top).min(cast::f32_to_u32(height));
            for py in y..y.saturating_add(top_height).min(y_end) {
                for px in x..x_end {
                    if px < self.width && py < self.height {
                        let bg = image.get_pixel(px, py);
                        let blended = Self::alpha_blend(*bg, color);
                        image.put_pixel(px, py, blended);
                    }
                }
            }
        }

        if border_width.bottom > 0.0 {
            let bottom_start = y_end.saturating_sub(cast::f32_to_u32(border_width.bottom));
            for py in bottom_start..y_end {
                for px in x..x_end {
                    if px < self.width && py < self.height {
                        let bg = image.get_pixel(px, py);
                        let blended = Self::alpha_blend(*bg, color);
                        image.put_pixel(px, py, blended);
                    }
                }
            }
        }

        if border_width.left > 0.0 {
            let left_width = cast::f32_to_u32(border_width.left).min(cast::f32_to_u32(width));
            for py in y..y_end {
                for px in x..x.saturating_add(left_width).min(x_end) {
                    if px < self.width && py < self.height {
                        let bg = image.get_pixel(px, py);
                        let blended = Self::alpha_blend(*bg, color);
                        image.put_pixel(px, py, blended);
                    }
                }
            }
        }

        if border_width.right > 0.0 {
            let right_start = x_end.saturating_sub(cast::f32_to_u32(border_width.right));
            for py in y..y_end {
                for px in right_start..x_end {
                    if px < self.width && py < self.height {
                        let bg = image.get_pixel(px, py);
                        let blended = Self::alpha_blend(*bg, color);
                        image.put_pixel(px, py, blended);
                    }
                }
            }
        }
    }

    #[expect(clippy::too_many_arguments)]
    fn draw_rounded_border_masked(
        &self,
        image: &mut RgbaImage,
        mask_memory: &mut MaskMemory,
        x_start: u32,
        y_start: u32,
        width: f32,
        height: f32,
        border_width: BorderWidth,
        color: image::Rgba<u8>,
        radius: BorderRadius,
    ) {
        let outer_path = build_rounded_rect_path(width, height, &radius, 0.0, 0.0);

        let inner_radius = radius.inset_by(&border_width);
        let inner_width = width - border_width.left - border_width.right;
        let inner_height = height - border_width.top - border_width.bottom;
        let inner_path = build_rounded_rect_path(
            inner_width,
            inner_height,
            &inner_radius,
            border_width.left,
            border_width.top,
        );

        let mut combined_path = outer_path;
        combined_path.extend(inner_path);

        let (mask, placement) = mask_memory.render_with_style(&combined_path, Fill::EvenOdd.into());

        for rel_y in 0..cast::f32_to_u32(height) {
            for rel_x in 0..cast::f32_to_u32(width) {
                let canvas_x = x_start + rel_x;
                let canvas_y = y_start + rel_y;

                if canvas_x >= self.width || canvas_y >= self.height {
                    continue;
                }

                let mask_x = rel_x.cast_signed() - placement.left;
                let mask_y = rel_y.cast_signed() - placement.top;

                let alpha = if mask_x >= 0
                    && mask_x < placement.width.cast_signed()
                    && mask_y >= 0
                    && mask_y < placement.height.cast_signed()
                {
                    mask[mask_index(
                        mask_x.cast_unsigned(),
                        mask_y.cast_unsigned(),
                        placement.width,
                    )]
                } else {
                    0
                };

                if alpha == 0 {
                    continue;
                }

                let bg = image.get_pixel(canvas_x, canvas_y);
                let blended = if alpha < 255 {
                    Self::blend_border_with_alpha(*bg, color, alpha)
                } else {
                    Self::alpha_blend(*bg, color)
                };
                image.put_pixel(canvas_x, canvas_y, blended);
            }
        }
    }

    fn blend_border_with_alpha(
        bg: image::Rgba<u8>,
        fg: image::Rgba<u8>,
        mask_alpha: u8,
    ) -> image::Rgba<u8> {
        let alpha = (f32::from(fg[3]) / 255.0) * (f32::from(mask_alpha) / 255.0);
        let inv_alpha = 1.0 - alpha;

        image::Rgba([
            cast::f32_to_u8(f32::from(fg[0]) * alpha + f32::from(bg[0]) * inv_alpha),
            cast::f32_to_u8(f32::from(fg[1]) * alpha + f32::from(bg[1]) * inv_alpha),
            cast::f32_to_u8(f32::from(fg[2]) * alpha + f32::from(bg[2]) * inv_alpha),
            255,
        ])
    }

    pub(super) fn parse_border_width(style: &rustc_hash::FxHashMap<String, String>) -> BorderWidth {
        if let Some(border) = style.get("border") {
            let parts: Vec<&str> = border.split_whitespace().collect();
            if let Some(width_str) = parts.first()
                && let Ok(width) = width_str.trim_end_matches("px").parse::<f32>()
            {
                return BorderWidth { top: width, right: width, bottom: width, left: width };
            }
        }

        if let Some(width_str) = style.get("borderWidth")
            && let Ok(width) = width_str.trim_end_matches("px").parse::<f32>()
        {
            return BorderWidth { top: width, right: width, bottom: width, left: width };
        }

        BorderWidth {
            top: style
                .get("borderTopWidth")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            right: style
                .get("borderRightWidth")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            bottom: style
                .get("borderBottomWidth")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            left: style
                .get("borderLeftWidth")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
        }
    }

    fn parse_border_color(style: &rustc_hash::FxHashMap<String, String>) -> image::Rgba<u8> {
        if let Some(color) = style.get("borderColor") {
            return Self::parse_color(color);
        }

        if let Some(border) = style.get("border") {
            let parts: Vec<&str> = border.split_whitespace().collect();
            if parts.len() >= 3 {
                return Self::parse_color(parts[2]);
            }
        }

        Self::parse_color("black")
    }

    pub(super) fn parse_border_radius(
        style: &rustc_hash::FxHashMap<String, String>,
    ) -> BorderRadius {
        if let Some(radius_str) = style.get("borderRadius")
            && let Ok(radius) = radius_str.trim_end_matches("px").parse::<f32>()
        {
            return BorderRadius {
                top_left: radius,
                top_right: radius,
                bottom_right: radius,
                bottom_left: radius,
            };
        }

        BorderRadius {
            top_left: style
                .get("borderTopLeftRadius")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            top_right: style
                .get("borderTopRightRadius")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            bottom_right: style
                .get("borderBottomRightRadius")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
            bottom_left: style
                .get("borderBottomLeftRadius")
                .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                .unwrap_or(0.0),
        }
    }
}
