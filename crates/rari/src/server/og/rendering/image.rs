use std::{string::String, time::Duration};

use image::{RgbaImage, imageops};
use reqwest::blocking::Client;

use super::{super::layout::ComputedLayout, border::BorderRadius, renderer::ImageRenderer};
use crate::utils::{cast, float};

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

impl ImageRenderer {
    pub(super) fn render_image(
        &self,
        layout: &ComputedLayout,
        canvas: &mut RgbaImage,
    ) -> Result<(), String> {
        let src = layout
            .element
            .props
            .get("src")
            .and_then(|v| v.as_str())
            .ok_or("Image element missing src attribute")?;

        let source_image = Self::load_image(src)?;

        let object_fit = layout.style.get("objectFit").map(String::as_str).unwrap_or("fill");

        let border_radius = Self::parse_border_radius(&layout.style);

        let target_width = cast::f32_to_u32(layout.width);
        let target_height = cast::f32_to_u32(layout.height);

        let (processed_image, offset_x, offset_y) =
            Self::process_object_fit(source_image, target_width, target_height, object_fit)?;

        let x_start = cast::f32_to_u32(layout.x) + offset_x;
        let y_start = cast::f32_to_u32(layout.y) + offset_y;

        if border_radius.top_left > 0.0
            || border_radius.top_right > 0.0
            || border_radius.bottom_right > 0.0
            || border_radius.bottom_left > 0.0
        {
            self.render_image_with_border_radius(
                &processed_image,
                canvas,
                x_start,
                y_start,
                border_radius,
            );
        } else {
            for (x, y, pixel) in processed_image.enumerate_pixels() {
                let canvas_x = x_start + x;
                let canvas_y = y_start + y;

                if canvas_x < self.width && canvas_y < self.height {
                    let bg = canvas.get_pixel(canvas_x, canvas_y);
                    let blended = Self::alpha_blend(*bg, *pixel);
                    canvas.put_pixel(canvas_x, canvas_y, blended);
                }
            }
        }

        Ok(())
    }

    fn process_object_fit(
        source_image: RgbaImage,
        target_width: u32,
        target_height: u32,
        object_fit: &str,
    ) -> Result<(RgbaImage, u32, u32), String> {
        let src_width = float::u32_to_f32(source_image.width());
        let src_height = float::u32_to_f32(source_image.height());
        let target_w = float::u32_to_f32(target_width);
        let target_h = float::u32_to_f32(target_height);

        match object_fit {
            "contain" => {
                let scale = (target_w / src_width).min(target_h / src_height);
                let new_width = cast::f32_to_u32(src_width * scale);
                let new_height = cast::f32_to_u32(src_height * scale);

                let resized = imageops::resize(
                    &source_image,
                    new_width,
                    new_height,
                    imageops::FilterType::CatmullRom,
                );

                let offset_x = (target_width - new_width) / 2;
                let offset_y = (target_height - new_height) / 2;

                Ok((resized, offset_x, offset_y))
            }
            "cover" => {
                let scale = (target_w / src_width).max(target_h / src_height);
                let new_width = cast::f32_to_u32(src_width * scale);
                let new_height = cast::f32_to_u32(src_height * scale);

                let resized = imageops::resize(
                    &source_image,
                    new_width,
                    new_height,
                    imageops::FilterType::CatmullRom,
                );

                let crop_x = (new_width - target_width) / 2;
                let crop_y = (new_height - target_height) / 2;

                let cropped =
                    imageops::crop_imm(&resized, crop_x, crop_y, target_width, target_height)
                        .to_image();

                Ok((cropped, 0, 0))
            }
            "scale-down" => {
                let scale = (target_w / src_width).min(target_h / src_height).min(1.0);
                let new_width = cast::f32_to_u32(src_width * scale);
                let new_height = cast::f32_to_u32(src_height * scale);

                let resized = if scale < 1.0 {
                    imageops::resize(
                        &source_image,
                        new_width,
                        new_height,
                        imageops::FilterType::CatmullRom,
                    )
                } else {
                    source_image
                };

                let offset_x = (target_width - new_width) / 2;
                let offset_y = (target_height - new_height) / 2;

                Ok((resized, offset_x, offset_y))
            }
            "none" => {
                let offset_x = if src_width < target_w {
                    cast::f32_to_u32((target_w - src_width) / 2.0)
                } else {
                    0
                };
                let offset_y = if src_height < target_h {
                    cast::f32_to_u32((target_h - src_height) / 2.0)
                } else {
                    0
                };

                if src_width > target_w || src_height > target_h {
                    let crop_x = if src_width > target_w {
                        cast::f32_to_u32((src_width - target_w) / 2.0)
                    } else {
                        0
                    };
                    let crop_y = if src_height > target_h {
                        cast::f32_to_u32((src_height - target_h) / 2.0)
                    } else {
                        0
                    };

                    let crop_width = cast::f32_to_u32(src_width.min(target_w));
                    let crop_height = cast::f32_to_u32(src_height.min(target_h));

                    let cropped =
                        imageops::crop_imm(&source_image, crop_x, crop_y, crop_width, crop_height)
                            .to_image();

                    Ok((cropped, offset_x, offset_y))
                } else {
                    Ok((source_image, offset_x, offset_y))
                }
            }
            _ => {
                let resized = if source_image.width() != target_width
                    || source_image.height() != target_height
                {
                    imageops::resize(
                        &source_image,
                        target_width,
                        target_height,
                        imageops::FilterType::CatmullRom,
                    )
                } else {
                    source_image
                };

                Ok((resized, 0, 0))
            }
        }
    }

    fn render_image_with_border_radius(
        &self,
        image: &RgbaImage,
        canvas: &mut RgbaImage,
        x_start: u32,
        y_start: u32,
        radius: BorderRadius,
    ) {
        let img_width = float::u32_to_f32(image.width());
        let img_height = float::u32_to_f32(image.height());

        for (x, y, pixel) in image.enumerate_pixels() {
            let canvas_x = x_start + x;
            let canvas_y = y_start + y;

            if canvas_x >= self.width || canvas_y >= self.height {
                continue;
            }

            let fx = float::u32_to_f32(x);
            let fy = float::u32_to_f32(y);

            let in_corner = if fx < radius.top_left && fy < radius.top_left {
                let dx = radius.top_left - fx;
                let dy = radius.top_left - fy;
                dx * dx + dy * dy <= radius.top_left * radius.top_left
            } else if fx >= img_width - radius.top_right && fy < radius.top_right {
                let dx = fx - (img_width - radius.top_right);
                let dy = radius.top_right - fy;
                dx * dx + dy * dy <= radius.top_right * radius.top_right
            } else if fx < radius.bottom_left && fy >= img_height - radius.bottom_left {
                let dx = radius.bottom_left - fx;
                let dy = fy - (img_height - radius.bottom_left);
                dx * dx + dy * dy <= radius.bottom_left * radius.bottom_left
            } else if fx >= img_width - radius.bottom_right
                && fy >= img_height - radius.bottom_right
            {
                let dx = fx - (img_width - radius.bottom_right);
                let dy = fy - (img_height - radius.bottom_right);
                dx * dx + dy * dy <= radius.bottom_right * radius.bottom_right
            } else {
                true
            };

            if in_corner {
                let bg = canvas.get_pixel(canvas_x, canvas_y);
                let blended = Self::alpha_blend(*bg, *pixel);
                canvas.put_pixel(canvas_x, canvas_y, blended);
            }
        }
    }

    fn load_image(src: &str) -> Result<RgbaImage, String> {
        if src.starts_with("http://") || src.starts_with("https://") {
            Self::load_remote_image(src)
        } else if src.starts_with("data:") {
            Self::load_data_url(src)
        } else {
            Ok(image::open(src).map_err(|e| format!("Failed to load image {src}: {e}"))?.to_rgba8())
        }
    }

    fn load_remote_image(url: &str) -> Result<RgbaImage, String> {
        use std::io::Read;

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let response =
            client.get(url).send().map_err(|e| format!("Failed to fetch image from {url}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch image: HTTP {}", response.status()));
        }

        let mut buffer = Vec::new();
        response
            .take((MAX_IMAGE_SIZE + 1) as u64)
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read image data: {e}"))?;

        if buffer.len() > MAX_IMAGE_SIZE {
            return Err("Image too large (max 10MB)".to_string());
        }

        Ok(image::load_from_memory(&buffer)
            .map_err(|e| format!("Failed to decode image: {e}"))?
            .to_rgba8())
    }

    fn load_data_url(data_url: &str) -> Result<RgbaImage, String> {
        let parts: Vec<&str> = data_url.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err("Invalid data URL format".to_string());
        }

        let header = parts[0];
        let data = parts[1];

        if header.contains("base64") {
            use base64::{Engine as _, engine::general_purpose};
            let decoded = general_purpose::STANDARD
                .decode(data)
                .map_err(|e| format!("Failed to decode base64: {e}"))?;

            Ok(image::load_from_memory(&decoded)
                .map_err(|e| format!("Failed to decode image: {e}"))?
                .to_rgba8())
        } else {
            Err("Only base64 data URLs are supported".to_string())
        }
    }
}
