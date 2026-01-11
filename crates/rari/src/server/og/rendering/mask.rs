use std::f32::consts::SQRT_2;
use zeno::{Command, Mask, PathBuilder, Placement, Scratch, Style};

use super::border::BorderRadius;

#[derive(Default)]
pub(super) struct MaskMemory {
    scratch: Scratch,
    buffer: Vec<u8>,
}

impl Clone for MaskMemory {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl MaskMemory {
    pub fn render(&mut self, paths: &[Command]) -> (&[u8], Placement) {
        self.render_with_style(paths, Style::default())
    }

    pub fn render_with_style(&mut self, paths: &[Command], style: Style) -> (&[u8], Placement) {
        let mut bounds = self.scratch.bounds(paths, style, None);

        bounds.min = bounds.min.floor();
        bounds.max = bounds.max.ceil();

        self.buffer.clear();
        self.buffer.resize((bounds.width() as usize) * (bounds.height() as usize), 0);

        let placement = Mask::with_scratch(paths, &mut self.scratch)
            .style(style)
            .render_into(&mut self.buffer, None);

        (self.buffer.as_slice(), placement)
    }
}

pub(super) fn build_rounded_rect_path(
    width: f32,
    height: f32,
    radius: &BorderRadius,
    offset_x: f32,
    offset_y: f32,
) -> Vec<Command> {
    let mut path = Vec::with_capacity(10);

    const KAPPA: f32 = 4.0 / 3.0 * (SQRT_2 - 1.0);

    let scale = 1.0f32
        .min(if radius.top_left + radius.top_right > width {
            width / (radius.top_left + radius.top_right)
        } else {
            1.0
        })
        .min(if radius.bottom_left + radius.bottom_right > width {
            width / (radius.bottom_left + radius.bottom_right)
        } else {
            1.0
        })
        .min(if radius.top_left + radius.bottom_left > height {
            height / (radius.top_left + radius.bottom_left)
        } else {
            1.0
        })
        .min(if radius.top_right + radius.bottom_right > height {
            height / (radius.top_right + radius.bottom_right)
        } else {
            1.0
        });

    let tl = radius.top_left * scale;
    let tr = radius.top_right * scale;
    let br = radius.bottom_right * scale;
    let bl = radius.bottom_left * scale;

    path.move_to((offset_x + tl.max(0.0), offset_y));

    path.line_to((offset_x + width - tr.max(0.0), offset_y));

    if tr > 0.0 {
        path.curve_to(
            (offset_x + width - tr * (1.0 - KAPPA), offset_y),
            (offset_x + width, offset_y + tr * (1.0 - KAPPA)),
            (offset_x + width, offset_y + tr),
        );
    }

    path.line_to((offset_x + width, offset_y + height - br.max(0.0)));

    if br > 0.0 {
        path.curve_to(
            (offset_x + width, offset_y + height - br * (1.0 - KAPPA)),
            (offset_x + width - br * (1.0 - KAPPA), offset_y + height),
            (offset_x + width - br, offset_y + height),
        );
    }

    path.line_to((offset_x + bl.max(0.0), offset_y + height));

    if bl > 0.0 {
        path.curve_to(
            (offset_x + bl * (1.0 - KAPPA), offset_y + height),
            (offset_x, offset_y + height - bl * (1.0 - KAPPA)),
            (offset_x, offset_y + height - bl),
        );
    }

    path.line_to((offset_x, offset_y + tl.max(0.0)));

    if tl > 0.0 {
        path.curve_to(
            (offset_x, offset_y + tl * (1.0 - KAPPA)),
            (offset_x + tl * (1.0 - KAPPA), offset_y),
            (offset_x + tl, offset_y),
        );
    }

    path.close();

    path
}

#[inline(always)]
pub(super) fn mask_index(x: u32, y: u32, width: u32) -> usize {
    (y * width + x) as usize
}
