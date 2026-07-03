use image::RgbaImage;

use super::{
    super::layout::{ComputedLayout, style::LinearGradient},
    mask::{MaskMemory, build_rounded_rect_path, mask_index},
    renderer::ImageRenderer,
};
use crate::utils::{cast, float};

impl ImageRenderer {
    pub(super) fn render_background(
        &self,
        layout: &ComputedLayout,
        bg: &str,
        image: &mut RgbaImage,
        mask_memory: &mut MaskMemory,
    ) -> Result<(), String> {
        let border_radius = Self::parse_border_radius(&layout.style);

        let x_start = cast::f32_to_u32(layout.x);
        let y_start = cast::f32_to_u32(layout.y);
        let box_width = layout.width;
        let box_height = layout.height;

        let has_radius = border_radius.top_left > 0.0
            || border_radius.top_right > 0.0
            || border_radius.bottom_right > 0.0
            || border_radius.bottom_left > 0.0;

        let mask_data: Option<(Vec<u8>, u32, u32, i32, i32)> = if has_radius {
            let path = build_rounded_rect_path(box_width, box_height, &border_radius, 0.0, 0.0);
            let (mask, placement) = mask_memory.render(&path);
            Some((mask.to_vec(), placement.width, placement.height, placement.left, placement.top))
        } else {
            None
        };

        if let Some(gradient) = LinearGradient::parse(bg) {
            let params = gradient.calculate_params(box_width, box_height);

            for rel_y in 0..cast::f32_to_u32(box_height) {
                for rel_x in 0..cast::f32_to_u32(box_width) {
                    let canvas_x = x_start + rel_x;
                    let canvas_y = y_start + rel_y;

                    if canvas_x >= self.width || canvas_y >= self.height {
                        continue;
                    }

                    let alpha = if let Some((ref mask, mask_w, mask_h, mask_left, mask_top)) =
                        mask_data
                    {
                        let mask_x = rel_x.cast_signed() - mask_left;
                        let mask_y = rel_y.cast_signed() - mask_top;

                        if mask_x >= 0
                            && mask_x < mask_w.cast_signed()
                            && mask_y >= 0
                            && mask_y < mask_h.cast_signed()
                        {
                            mask[mask_index(mask_x.cast_unsigned(), mask_y.cast_unsigned(), mask_w)]
                        } else {
                            0
                        }
                    } else {
                        255
                    };

                    if alpha == 0 {
                        continue;
                    }

                    let dx = float::u32_to_f32(rel_x) - params.cx;
                    let dy = float::u32_to_f32(rel_y) - params.cy;
                    let projection = dx * params.dir_x + dy * params.dir_y;
                    let position =
                        ((projection + params.max_extent) / params.axis_length).clamp(0.0, 1.0);

                    let mut color = gradient.color_at(position, params.axis_length);

                    if alpha < 255 {
                        let bg_pixel = image.get_pixel(canvas_x, canvas_y);
                        color = Self::blend_with_alpha(*bg_pixel, color, alpha);
                    }

                    image.put_pixel(canvas_x, canvas_y, color);
                }
            }
        } else {
            let color = Self::parse_color(bg);

            for rel_y in 0..cast::f32_to_u32(box_height) {
                for rel_x in 0..cast::f32_to_u32(box_width) {
                    let canvas_x = x_start + rel_x;
                    let canvas_y = y_start + rel_y;

                    if canvas_x >= self.width || canvas_y >= self.height {
                        continue;
                    }

                    let alpha = if let Some((ref mask, mask_w, mask_h, mask_left, mask_top)) =
                        mask_data
                    {
                        let mask_x = rel_x.cast_signed() - mask_left;
                        let mask_y = rel_y.cast_signed() - mask_top;

                        if mask_x >= 0
                            && mask_x < mask_w.cast_signed()
                            && mask_y >= 0
                            && mask_y < mask_h.cast_signed()
                        {
                            mask[mask_index(mask_x.cast_unsigned(), mask_y.cast_unsigned(), mask_w)]
                        } else {
                            0
                        }
                    } else {
                        255
                    };

                    if alpha == 0 {
                        continue;
                    }

                    let final_color = if alpha < 255 {
                        let bg_pixel = image.get_pixel(canvas_x, canvas_y);
                        Self::blend_with_alpha(*bg_pixel, color, alpha)
                    } else {
                        color
                    };

                    image.put_pixel(canvas_x, canvas_y, final_color);
                }
            }
        }

        Ok(())
    }

    fn blend_with_alpha(
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
}
