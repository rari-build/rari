use parley::{
    FontContext as ParleyFontContext, GenericFamily,
    fontique::{Blob, Collection, CollectionOptions, FallbackKey, Script},
};
use std::borrow::Cow;
use std::sync::Arc;
use thiserror::Error;

const TWEMOJI_FONT: &[u8] = include_bytes!("fonts/TwemojiMozilla-colr.woff2");
const NOTO_SANS_FONT: &[u8] = include_bytes!("fonts/NotoSansTC-VariableFont_wght.woff2");

#[derive(Debug, Error)]
pub enum FontError {
    #[error("Error occurred during WOFF2 decompression: {0}")]
    Woff2(String),
    #[error("Unsupported font format")]
    UnsupportedFormat,
}

#[derive(Copy, Clone)]
enum FontFormat {
    Woff2,
    Woff,
    Ttf,
    Otf,
}

pub fn load_font(source: &[u8]) -> Result<Cow<'_, [u8]>, FontError> {
    let format = guess_font_format(source)?;

    match format {
        FontFormat::Ttf | FontFormat::Otf => Ok(Cow::Borrowed(source)),
        FontFormat::Woff2 => {
            let ttf =
                wuff::decompress_woff2(source).map_err(|e| FontError::Woff2(format!("{:?}", e)))?;
            Ok(Cow::Owned(ttf))
        }
        FontFormat::Woff => {
            let ttf =
                wuff::decompress_woff1(source).map_err(|e| FontError::Woff2(format!("{:?}", e)))?;
            Ok(Cow::Owned(ttf))
        }
    }
}

fn guess_font_format(source: &[u8]) -> Result<FontFormat, FontError> {
    if source.len() < 4 {
        return Err(FontError::UnsupportedFormat);
    }

    match &source[0..4] {
        b"wOF2" => Ok(FontFormat::Woff2),
        b"wOFF" => Ok(FontFormat::Woff),
        [0x00, 0x01, 0x00, 0x00] => Ok(FontFormat::Ttf),
        b"OTTO" => Ok(FontFormat::Otf),
        _ => Err(FontError::UnsupportedFormat),
    }
}

#[derive(Clone)]
pub struct FontContext {
    pub(crate) inner: ParleyFontContext,
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}

impl FontContext {
    pub fn new() -> Self {
        let inner = ParleyFontContext {
            collection: Collection::new(CollectionOptions { system_fonts: false, shared: false }),
            source_cache: Default::default(),
        };

        let mut ctx = Self { inner };

        if let Err(e) = ctx.load_twemoji() {
            eprintln!("Warning: Failed to load Twemoji font: {:?}", e);
        }

        if let Err(e) = ctx.load_default_font() {
            eprintln!("Warning: Failed to load default font: {:?}", e);
        }

        ctx
    }

    fn load_default_font(&mut self) -> Result<(), FontError> {
        let font_data = load_font(NOTO_SANS_FONT)?;
        let blob = Blob::new(Arc::new(font_data.into_owned()));

        let fonts = self.inner.collection.register_fonts(blob, None);

        for (family, _) in fonts {
            self.inner
                .collection
                .append_generic_families(GenericFamily::SansSerif, std::iter::once(family));
        }

        Ok(())
    }

    fn load_twemoji(&mut self) -> Result<(), FontError> {
        let font_data = load_font(TWEMOJI_FONT)?;
        let blob = Blob::new(Arc::new(font_data.into_owned()));

        let fonts = self.inner.collection.register_fonts(blob, None);

        for (family, _) in fonts {
            self.inner
                .collection
                .append_generic_families(GenericFamily::Emoji, std::iter::once(family));

            for (script, _) in Script::all_samples() {
                self.inner
                    .collection
                    .append_fallbacks(FallbackKey::new(*script, None), std::iter::once(family));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_context_creation() {
        let _ctx = FontContext::new();
    }

    #[test]
    fn test_load_twemoji() {
        let _ctx = FontContext::new();
    }

    #[test]
    fn test_emoji_text_layout() {
        use parley::{LayoutContext, TextStyle};

        let ctx = FontContext::new();
        let mut font_ctx = ctx.inner.clone();
        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();

        let root_style = TextStyle { font_size: 32.0, ..Default::default() };

        let mut builder = layout_cx.tree_builder(&mut font_ctx, 1.0, true, &root_style);
        builder.push_text("Hello 🎉 World");
        let (mut layout, _text) = builder.build();
        layout.break_all_lines(Some(500.0));

        let line_count = layout.lines().count();
        assert!(line_count > 0, "Should have at least one line");

        let mut glyph_count = 0;
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                    glyph_count += gr.positioned_glyphs().count();
                }
            }
        }

        println!("Line count: {}, Glyph count: {}", line_count, glyph_count);
        assert!(glyph_count > 0, "Should have glyphs");
    }

    #[test]
    fn test_emoji_glyph_rendering() {
        use parley::{LayoutContext, TextStyle};
        use swash::{FontRef, scale::ScaleContext};

        let ctx = FontContext::new();
        let mut font_ctx = ctx.inner.clone();
        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();

        let root_style = TextStyle { font_size: 32.0, ..Default::default() };

        let mut builder = layout_cx.tree_builder(&mut font_ctx, 1.0, true, &root_style);
        builder.push_text("🎉");
        let (mut layout, _text) = builder.build();
        layout.break_all_lines(Some(500.0));

        let mut scale_context = ScaleContext::new();

        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                    let run = gr.run();
                    let font_ref =
                        FontRef::from_index(run.font().data.as_ref(), run.font().index as usize)
                            .expect("Invalid font index");

                    println!("Font has {} color palettes", font_ref.color_palettes().count());

                    let mut scaler = scale_context
                        .builder(font_ref)
                        .size(32.0)
                        .normalized_coords(run.normalized_coords())
                        .build();

                    for glyph in gr.positioned_glyphs() {
                        println!("Glyph ID: {}", glyph.id);

                        if let Some(bitmap) = scaler
                            .scale_color_bitmap(glyph.id as u16, swash::scale::StrikeWith::BestFit)
                        {
                            println!(
                                "  -> Color bitmap: {}x{}",
                                bitmap.placement.width, bitmap.placement.height
                            );
                        } else if let Some(outline) = scaler.scale_color_outline(glyph.id as u16) {
                            println!(
                                "  -> Color outline with {} layers, is_color: {}",
                                outline.len(),
                                outline.is_color()
                            );
                        } else if let Some(_outline) = scaler.scale_outline(glyph.id as u16) {
                            println!("  -> Regular outline");
                        } else {
                            println!("  -> NO GLYPH DATA!");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_guess_font_format() {
        assert!(matches!(guess_font_format(b"wOF2...."), Ok(FontFormat::Woff2)));
        assert!(matches!(guess_font_format(b"wOFF...."), Ok(FontFormat::Woff)));
        assert!(matches!(guess_font_format(&[0x00, 0x01, 0x00, 0x00]), Ok(FontFormat::Ttf)));
        assert!(matches!(guess_font_format(b"OTTO"), Ok(FontFormat::Otf)));
        assert!(matches!(guess_font_format(b"invalid"), Err(FontError::UnsupportedFormat)));
        assert!(matches!(guess_font_format(b"abc"), Err(FontError::UnsupportedFormat)));
    }
}
