use super::super::layout::ComputedLayout;
use super::super::layout::style::LinearGradient;
use super::mask::{MaskMemory, build_rounded_rect_path, mask_index};
use super::renderer::ImageRenderer;
use image::RgbaImage;

impl ImageRenderer {
    pub(super) fn render_background(
        &self,
        layout: &ComputedLayout,
        bg: &str,
        image: &mut RgbaImage,
        mask_memory: &mut MaskMemory,
    ) -> Result<(), String> {
        let border_radius = self.parse_border_radius(&layout.style);

        let x_start = layout.x as u32;
        let y_start = layout.y as u32;
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

            for rel_y in 0..box_height as u32 {
                for rel_x in 0..box_width as u32 {
                    let canvas_x = x_start + rel_x;
                    let canvas_y = y_start + rel_y;

                    if canvas_x >= self.width || canvas_y >= self.height {
                        continue;
                    }

                    let alpha =
                        if let Some((ref mask, mask_w, mask_h, mask_left, mask_top)) = mask_data {
                            let mask_x = rel_x as i32 - mask_left;
                            let mask_y = rel_y as i32 - mask_top;

                            if mask_x >= 0
                                && mask_x < mask_w as i32
                                && mask_y >= 0
                                && mask_y < mask_h as i32
                            {
                                mask[mask_index(mask_x as u32, mask_y as u32, mask_w)]
                            } else {
                                0
                            }
                        } else {
                            255
                        };

                    if alpha == 0 {
                        continue;
                    }

                    let dx = rel_x as f32 - params.cx;
                    let dy = rel_y as f32 - params.cy;
                    let projection = dx * params.dir_x + dy * params.dir_y;
                    let position =
                        ((projection + params.max_extent) / params.axis_length).clamp(0.0, 1.0);

                    let mut color = gradient.color_at(position, params.axis_length);

                    if alpha < 255 {
                        let bg_pixel = image.get_pixel(canvas_x, canvas_y);
                        color = self.blend_with_alpha(*bg_pixel, color, alpha);
                    }

                    image.put_pixel(canvas_x, canvas_y, color);
                }
            }
        } else {
            let color = self.parse_color(bg);

            for rel_y in 0..box_height as u32 {
                for rel_x in 0..box_width as u32 {
                    let canvas_x = x_start + rel_x;
                    let canvas_y = y_start + rel_y;

                    if canvas_x >= self.width || canvas_y >= self.height {
                        continue;
                    }

                    let alpha =
                        if let Some((ref mask, mask_w, mask_h, mask_left, mask_top)) = mask_data {
                            let mask_x = rel_x as i32 - mask_left;
                            let mask_y = rel_y as i32 - mask_top;

                            if mask_x >= 0
                                && mask_x < mask_w as i32
                                && mask_y >= 0
                                && mask_y < mask_h as i32
                            {
                                mask[mask_index(mask_x as u32, mask_y as u32, mask_w)]
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
                        self.blend_with_alpha(*bg_pixel, color, alpha)
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
        &self,
        bg: image::Rgba<u8>,
        fg: image::Rgba<u8>,
        mask_alpha: u8,
    ) -> image::Rgba<u8> {
        let alpha = (fg[3] as f32 / 255.0) * (mask_alpha as f32 / 255.0);
        let inv_alpha = 1.0 - alpha;

        image::Rgba([
            ((fg[0] as f32 * alpha + bg[0] as f32 * inv_alpha) as u8),
            ((fg[1] as f32 * alpha + bg[1] as f32 * inv_alpha) as u8),
            ((fg[2] as f32 * alpha + bg[2] as f32 * inv_alpha) as u8),
            255,
        ])
    }
}
