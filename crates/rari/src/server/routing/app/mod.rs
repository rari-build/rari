pub(crate) mod cache;
mod render;
mod route;

pub(crate) use render::{collect_page_metadata, wrap_html_with_metadata};
pub use render::{
    render_fallback_html, render_rsc_navigation_streaming, render_streaming_with_layout,
    render_synchronous, render_with_fallback,
};
pub use route::handle_app_route;
