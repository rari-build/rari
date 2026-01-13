use super::super::layout::ComputedLayout;
use super::super::types::JsxChild;
use super::renderer::ImageRenderer;
use cow_utils::CowUtils;
use image::{Rgba, RgbaImage};
use parley::{Alignment, LayoutContext};
use swash::{
    FontRef,
    scale::{ScaleContext, StrikeWith},
};
use zeno::{Mask, PathData};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextDecoration {
    Underline,
    LineThrough,
    Overline,
}

pub(super) struct GlyphRenderParams {
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub font_weight: u16,
    pub color: Rgba<u8>,
    pub max_width: Option<f32>,
    pub line_height: f32,
    pub text_align: Alignment,
    pub text_decoration: Vec<TextDecoration>,
}

impl ImageRenderer {
    pub(super) fn render_text(
        &mut self,
        layout: &ComputedLayout,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        let text = self.extract_text(&layout.element);
        if text.is_empty() {
            return Ok(());
        }

        let font_size =
            layout.style.get("fontSize").and_then(|s| s.parse::<f32>().ok()).unwrap_or(16.0);

        let color =
            layout.style.get("color").map(|c| self.parse_color(c)).unwrap_or(Rgba([0, 0, 0, 255]));

        let font_weight = self.parse_font_weight(&layout.style);

        let line_height = self.parse_line_height(&layout.style, font_size);

        let text_align = self.parse_text_align(&layout.style);

        let text_decoration = self.parse_text_decoration(&layout.style);

        let params = GlyphRenderParams {
            x: layout.x + layout.border.left + layout.padding.left,
            y: layout.y + layout.border.top + layout.padding.top,
            font_size,
            font_weight,
            color,
            max_width: Some(
                layout.width
                    - layout.border.left
                    - layout.border.right
                    - layout.padding.left
                    - layout.padding.right,
            ),
            line_height,
            text_align,
            text_decoration,
        };

        self.render_glyphs(&text, &params, image)?;

        Ok(())
    }

    fn extract_text(&self, element: &super::super::types::JsxElement) -> String {
        element
            .children
            .iter()
            .filter_map(|child| match child {
                JsxChild::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn parse_line_height(
        &self,
        style: &rustc_hash::FxHashMap<String, String>,
        font_size: f32,
    ) -> f32 {
        if let Some(lh) = style.get("lineHeight") {
            if let Ok(multiplier) = lh.parse::<f32>() {
                return font_size * multiplier;
            }

            if lh.ends_with("px")
                && let Ok(px) = lh.trim_end_matches("px").parse::<f32>()
            {
                return px;
            }

            if lh.ends_with("em")
                && let Ok(em) = lh.trim_end_matches("em").parse::<f32>()
            {
                return font_size * em;
            }

            if lh.ends_with('%')
                && let Ok(pct) = lh.trim_end_matches('%').parse::<f32>()
            {
                return font_size * (pct / 100.0);
            }

            if lh == "normal" {
                return font_size * 1.2;
            }
        }

        font_size * 1.2
    }

    fn parse_text_align(&self, style: &rustc_hash::FxHashMap<String, String>) -> Alignment {
        style
            .get("textAlign")
            .map(|ta| match ta.as_str() {
                "left" | "start" => Alignment::Start,
                "right" | "end" => Alignment::End,
                "center" => Alignment::Center,
                "justify" => Alignment::Justify,
                _ => Alignment::Start,
            })
            .unwrap_or(Alignment::Start)
    }

    fn parse_text_decoration(
        &self,
        style: &rustc_hash::FxHashMap<String, String>,
    ) -> Vec<TextDecoration> {
        let mut decorations = Vec::new();

        if let Some(td) = style.get("textDecoration") {
            let td_lower = td.cow_to_lowercase();
            if td_lower.contains("underline") {
                decorations.push(TextDecoration::Underline);
            }
            if td_lower.contains("line-through") {
                decorations.push(TextDecoration::LineThrough);
            }
            if td_lower.contains("overline") {
                decorations.push(TextDecoration::Overline);
            }
        }

        if let Some(tdl) = style.get("textDecorationLine") {
            let tdl_lower = tdl.cow_to_lowercase();
            if tdl_lower.contains("underline") && !decorations.contains(&TextDecoration::Underline)
            {
                decorations.push(TextDecoration::Underline);
            }
            if tdl_lower.contains("line-through")
                && !decorations.contains(&TextDecoration::LineThrough)
            {
                decorations.push(TextDecoration::LineThrough);
            }
            if tdl_lower.contains("overline") && !decorations.contains(&TextDecoration::Overline) {
                decorations.push(TextDecoration::Overline);
            }
        }

        decorations
    }

    fn render_glyphs(
        &mut self,
        text: &str,
        params: &GlyphRenderParams,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        use parley::TextStyle;

        let line_height_parley = parley::LineHeight::Absolute(params.line_height);

        let root_style = TextStyle {
            font_size: params.font_size,
            font_weight: parley::style::FontWeight::new(params.font_weight as f32),
            line_height: line_height_parley,
            ..Default::default()
        };

        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();
        let mut builder = layout_cx.tree_builder(&mut self.font_context, 1.0, true, &root_style);

        builder.push_text(text);

        let (mut layout, _text) = builder.build();
        layout.break_all_lines(params.max_width);

        layout.align(params.max_width, params.text_align, Default::default());

        let mut scale_context = ScaleContext::new();

        for line in layout.lines() {
            if !params.text_decoration.is_empty() {
                self.draw_text_decorations(&line, params, image)?;
            }

            for item in line.items() {
                let glyph_run = match item {
                    parley::PositionedLayoutItem::GlyphRun(gr) => gr,
                    _ => continue,
                };

                let run = glyph_run.run();
                let font_ref =
                    FontRef::from_index(run.font().data.as_ref(), run.font().index as usize)
                        .ok_or("Invalid font index")?;

                let mut scaler = scale_context
                    .builder(font_ref)
                    .size(params.font_size)
                    .normalized_coords(run.normalized_coords())
                    .build();

                let palette = font_ref.color_palettes().next();

                for glyph in glyph_run.positioned_glyphs() {
                    let glyph_x = params.x + glyph.x;
                    let glyph_y = params.y + glyph.y;

                    if let Some(bitmap) =
                        scaler.scale_color_bitmap(glyph.id as u16, StrikeWith::BestFit)
                    {
                        self.draw_color_bitmap(&bitmap, glyph_x, glyph_y, image);
                    } else if let Some(outline) = scaler.scale_color_outline(glyph.id as u16) {
                        self.draw_color_outline(&outline, glyph_x, glyph_y, palette, image)?;
                    } else if let Some(outline) = scaler.scale_outline(glyph.id as u16) {
                        self.draw_outline(&outline, glyph_x, glyph_y, params.color, image)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn draw_text_decorations(
        &self,
        line: &parley::Line<[u8; 4]>,
        params: &GlyphRenderParams,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        let metrics = line.metrics();
        let line_y = params.y + metrics.baseline;

        let mut line_start_x = f32::MAX;
        let mut line_end_x = 0.0f32;

        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                let run_x = gr.offset();
                let run_width = gr.advance();
                line_start_x = line_start_x.min(run_x);
                line_end_x = line_end_x.max(run_x + run_width);
            }
        }

        if line_start_x == f32::MAX {
            return Ok(());
        }

        let decoration_thickness = (params.font_size / 18.0).max(1.0);

        for decoration in &params.text_decoration {
            let y_offset = match decoration {
                TextDecoration::Underline => line_y + metrics.descent * 0.3,
                TextDecoration::LineThrough => line_y - metrics.ascent * 0.35,
                TextDecoration::Overline => line_y - metrics.ascent - decoration_thickness,
            };

            self.draw_decoration_line(
                params.x + line_start_x,
                y_offset,
                line_end_x - line_start_x,
                decoration_thickness,
                params.color,
                image,
            );
        }

        Ok(())
    }

    fn draw_decoration_line(
        &self,
        x: f32,
        y: f32,
        width: f32,
        thickness: f32,
        color: Rgba<u8>,
        image: &mut RgbaImage,
    ) {
        let x_start = x.max(0.0) as u32;
        let x_end = ((x + width) as u32).min(self.width);
        let y_start = y.max(0.0) as u32;
        let y_end = ((y + thickness) as u32).min(self.height);

        for py in y_start..y_end {
            for px in x_start..x_end {
                if px < self.width && py < self.height {
                    let bg = image.get_pixel(px, py);
                    let blended = self.alpha_blend(*bg, color);
                    image.put_pixel(px, py, blended);
                }
            }
        }
    }

    fn draw_color_bitmap(
        &self,
        bitmap: &swash::scale::image::Image,
        x: f32,
        y: f32,
        image: &mut RgbaImage,
    ) {
        let placement = &bitmap.placement;
        let data = &bitmap.data;

        for row in 0..placement.height {
            for col in 0..placement.width {
                let pixel_x = (x + placement.left as f32 + col as f32) as i32;
                let pixel_y = (y - placement.top as f32 + row as f32) as i32;

                if pixel_x >= 0
                    && pixel_x < self.width as i32
                    && pixel_y >= 0
                    && pixel_y < self.height as i32
                {
                    let idx = ((row * placement.width + col) * 4) as usize;
                    if idx + 3 < data.len() {
                        let pixel = Rgba([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]);

                        if pixel[3] > 0 {
                            let bg = image.get_pixel(pixel_x as u32, pixel_y as u32);
                            let blended = self.alpha_blend(*bg, pixel);
                            image.put_pixel(pixel_x as u32, pixel_y as u32, blended);
                        }
                    }
                }
            }
        }
    }

    fn draw_color_outline(
        &mut self,
        outline: &swash::scale::outline::Outline,
        x: f32,
        y: f32,
        palette: Option<swash::ColorPalette>,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        use zeno::Command;

        if let Some(palette) = palette {
            for i in 0..outline.len() {
                let Some(layer) = outline.get(i) else {
                    break;
                };

                let Some(color_index) = layer.color_index() else {
                    continue;
                };

                let color_value = palette.get(color_index);
                let color = Rgba([color_value[0], color_value[1], color_value[2], color_value[3]]);

                let path_commands: Vec<Command> = layer
                    .path()
                    .commands()
                    .map(|cmd| match cmd {
                        Command::MoveTo(p) => Command::MoveTo((p.x, -p.y).into()),
                        Command::LineTo(p) => Command::LineTo((p.x, -p.y).into()),
                        Command::CurveTo(p1, p2, p3) => Command::CurveTo(
                            (p1.x, -p1.y).into(),
                            (p2.x, -p2.y).into(),
                            (p3.x, -p3.y).into(),
                        ),
                        Command::QuadTo(p1, p2) => {
                            Command::QuadTo((p1.x, -p1.y).into(), (p2.x, -p2.y).into())
                        }
                        Command::Close => Command::Close,
                    })
                    .collect();

                self.draw_path_commands(&path_commands, x, y, color, image)?;
            }
        }

        Ok(())
    }

    fn draw_outline(
        &mut self,
        outline: &swash::scale::outline::Outline,
        x: f32,
        y: f32,
        color: Rgba<u8>,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        use zeno::Command;

        let path_commands: Vec<Command> = outline
            .path()
            .commands()
            .map(|cmd| match cmd {
                Command::MoveTo(p) => Command::MoveTo((p.x, -p.y).into()),
                Command::LineTo(p) => Command::LineTo((p.x, -p.y).into()),
                Command::CurveTo(p1, p2, p3) => Command::CurveTo(
                    (p1.x, -p1.y).into(),
                    (p2.x, -p2.y).into(),
                    (p3.x, -p3.y).into(),
                ),
                Command::QuadTo(p1, p2) => {
                    Command::QuadTo((p1.x, -p1.y).into(), (p2.x, -p2.y).into())
                }
                Command::Close => Command::Close,
            })
            .collect();

        self.draw_path_commands(&path_commands, x, y, color, image)
    }

    fn draw_path_commands(
        &mut self,
        path_commands: &[zeno::Command],
        x: f32,
        y: f32,
        color: Rgba<u8>,
        image: &mut RgbaImage,
    ) -> Result<(), String> {
        use zeno::Transform;

        let transform = Transform::translation(x, y);
        let mut buffer = vec![0u8; self.width as usize * self.height as usize];

        let placement = Mask::with_scratch(path_commands, &mut self.scratch)
            .transform(Some(transform))
            .render_into(&mut buffer, None);

        for row in 0..placement.height {
            for col in 0..placement.width {
                let mask_x = placement.left + col as i32;
                let mask_y = placement.top + row as i32;

                if mask_x >= 0
                    && mask_x < self.width as i32
                    && mask_y >= 0
                    && mask_y < self.height as i32
                {
                    let idx = (row * placement.width + col) as usize;
                    if idx < buffer.len() {
                        let alpha = buffer[idx];

                        if alpha > 0 {
                            let bg = image.get_pixel(mask_x as u32, mask_y as u32);
                            let fg = Rgba([color[0], color[1], color[2], alpha]);
                            let blended = self.alpha_blend(*bg, fg);
                            image.put_pixel(mask_x as u32, mask_y as u32, blended);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
