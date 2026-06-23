#![expect(
    clippy::unnecessary_wraps,
    reason = "Rendering helper methods return Result for API consistency"
)]

mod background;
mod border;
mod image;
mod mask;
mod renderer;
mod svg;
mod text;

pub use renderer::ImageRenderer;
pub use svg::is_svg_element;
