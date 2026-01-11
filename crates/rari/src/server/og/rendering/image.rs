use super::super::layout::ComputedLayout;
use super::border::BorderRadius;
use super::renderer::ImageRenderer;
use image::RgbaImage;

impl ImageRenderer {
    pub(super) fn render_image(
        &mut self,
        layout: &ComputedLayout,
        canvas: &mut RgbaImage,
    ) -> Result<(), String> {
        let src = layout
            .element
            .props
            .get("src")
            .and_then(|v| v.as_str())
            .ok_or("Image element missing src attribute")?;

        let source_image = self.load_image(src)?;

        let object_fit = layout.style.get("objectFit").map(|s| s.as_str()).unwrap_or("fill");

        let border_radius = self.parse_border_radius(&layout.style);

        let target_width = layout.width as u32;
        let target_height = layout.height as u32;

        let (processed_image, offset_x, offset_y) =
            self.process_object_fit(source_image, target_width, target_height, object_fit)?;

        let x_start = layout.x as u32 + offset_x;
        let y_start = layout.y as u32 + offset_y;

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
                    let blended = self.alpha_blend(*bg, *pixel);
                    canvas.put_pixel(canvas_x, canvas_y, blended);
                }
            }
        }

        Ok(())
    }

    fn process_object_fit(
        &self,
        source_image: RgbaImage,
        target_width: u32,
        target_height: u32,
        object_fit: &str,
    ) -> Result<(RgbaImage, u32, u32), String> {
        let src_width = source_image.width() as f32;
        let src_height = source_image.height() as f32;
        let target_w = target_width as f32;
        let target_h = target_height as f32;

        match object_fit {
            "contain" => {
                let scale = (target_w / src_width).min(target_h / src_height);
                let new_width = (src_width * scale) as u32;
                let new_height = (src_height * scale) as u32;

                let resized = image::imageops::resize(
                    &source_image,
                    new_width,
                    new_height,
                    image::imageops::FilterType::CatmullRom,
                );

                let offset_x = (target_width - new_width) / 2;
                let offset_y = (target_height - new_height) / 2;

                Ok((resized, offset_x, offset_y))
            }
            "cover" => {
                let scale = (target_w / src_width).max(target_h / src_height);
                let new_width = (src_width * scale) as u32;
                let new_height = (src_height * scale) as u32;

                let resized = image::imageops::resize(
                    &source_image,
                    new_width,
                    new_height,
                    image::imageops::FilterType::CatmullRom,
                );

                let crop_x = (new_width - target_width) / 2;
                let crop_y = (new_height - target_height) / 2;

                let cropped = image::imageops::crop_imm(
                    &resized,
                    crop_x,
                    crop_y,
                    target_width,
                    target_height,
                )
                .to_image();

                Ok((cropped, 0, 0))
            }
            "scale-down" => {
                let scale = (target_w / src_width).min(target_h / src_height).min(1.0);
                let new_width = (src_width * scale) as u32;
                let new_height = (src_height * scale) as u32;

                let resized = if scale < 1.0 {
                    image::imageops::resize(
                        &source_image,
                        new_width,
                        new_height,
                        image::imageops::FilterType::CatmullRom,
                    )
                } else {
                    source_image
                };

                let offset_x = (target_width - new_width) / 2;
                let offset_y = (target_height - new_height) / 2;

                Ok((resized, offset_x, offset_y))
            }
            "none" => {
                let offset_x =
                    if src_width < target_w { ((target_w - src_width) / 2.0) as u32 } else { 0 };
                let offset_y =
                    if src_height < target_h { ((target_h - src_height) / 2.0) as u32 } else { 0 };

                if src_width > target_w || src_height > target_h {
                    let crop_x = if src_width > target_w {
                        ((src_width - target_w) / 2.0) as u32
                    } else {
                        0
                    };
                    let crop_y = if src_height > target_h {
                        ((src_height - target_h) / 2.0) as u32
                    } else {
                        0
                    };

                    let crop_width = src_width.min(target_w) as u32;
                    let crop_height = src_height.min(target_h) as u32;

                    let cropped = image::imageops::crop_imm(
                        &source_image,
                        crop_x,
                        crop_y,
                        crop_width,
                        crop_height,
                    )
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
                    image::imageops::resize(
                        &source_image,
                        target_width,
                        target_height,
                        image::imageops::FilterType::CatmullRom,
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
        let img_width = image.width() as f32;
        let img_height = image.height() as f32;

        for (x, y, pixel) in image.enumerate_pixels() {
            let canvas_x = x_start + x;
            let canvas_y = y_start + y;

            if canvas_x >= self.width || canvas_y >= self.height {
                continue;
            }

            let fx = x as f32;
            let fy = y as f32;

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
                let blended = self.alpha_blend(*bg, *pixel);
                canvas.put_pixel(canvas_x, canvas_y, blended);
            }
        }
    }

    fn load_image(&self, src: &str) -> Result<RgbaImage, String> {
        if src.starts_with("http://") || src.starts_with("https://") {
            self.load_remote_image(src)
        } else if src.starts_with("data:") {
            self.load_data_url(src)
        } else {
            Ok(image::open(src)
                .map_err(|e| format!("Failed to load image {}: {}", src, e))?
                .to_rgba8())
        }
    }

    fn load_remote_image(&self, url: &str) -> Result<RgbaImage, String> {
        use std::io::Read;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch image from {}: {}", url, e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch image: HTTP {}", response.status()));
        }

        const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
        let mut buffer = Vec::new();
        response
            .take(MAX_IMAGE_SIZE as u64)
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read image data: {}", e))?;

        if buffer.len() >= MAX_IMAGE_SIZE {
            return Err("Image too large (max 10MB)".to_string());
        }

        Ok(image::load_from_memory(&buffer)
            .map_err(|e| format!("Failed to decode image: {}", e))?
            .to_rgba8())
    }

    fn load_data_url(&self, data_url: &str) -> Result<RgbaImage, String> {
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
                .map_err(|e| format!("Failed to decode base64: {}", e))?;

            Ok(image::load_from_memory(&decoded)
                .map_err(|e| format!("Failed to decode image: {}", e))?
                .to_rgba8())
        } else {
            Err("Only base64 data URLs are supported".to_string())
        }
    }
}
