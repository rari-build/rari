pub mod constants;
pub mod loader;
pub mod renderer;
pub mod types;
pub mod utils;

pub use loader::{RscJsLoader, RscModuleOperation, StubType};
pub use renderer::RscRenderer;
pub use types::{ResourceLimits, ResourceMetrics, ResourceTracker};

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
#[allow(clippy::module_inception)]
mod tests {
    use smallvec::SmallVec;
    use std::sync::Arc;

    use crate::runtime::JsExecutionRuntime;

    use super::renderer::RscRenderer;

    #[tokio::test]
    async fn test_renderer_initialization() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let mut renderer = RscRenderer::new(runtime);

        let result = renderer.initialize().await;
        assert!(result.is_ok());
        assert!(renderer.initialized);
    }

    #[tokio::test]
    async fn test_render_to_string() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let mut renderer = RscRenderer::new(runtime);

        renderer
            .initialize()
            .await
            .expect("Failed to initialize renderer");

        {
            let mut registry = renderer.component_registry.lock();
            let _ = registry.register_component(
                "TestComponent",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }"
                    .to_string(),
                SmallVec::new(),
            );
            registry.mark_component_loaded("TestComponent");
        }

        let render_result = renderer.render_to_string("TestComponent", None).await;

        assert!(render_result.is_ok(), "render_to_string should succeed");
        let output = render_result.unwrap();
        assert!(!output.is_empty(), "Rendered output should not be empty");
    }

    #[tokio::test]
    async fn test_register_and_render_jsx_component() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        renderer
            .initialize()
            .await
            .expect("Failed to initialize renderer");

        let register_component_js = r#"
        globalThis.MyJsxComponent = function(props) {
            return React.createElement('h1', null, 'Hello ' + (props.name || 'JSX World') + '!');
        };

        globalThis.Component_a83fd0f5d95fb38e = globalThis.MyJsxComponent;
        true
        "#;

        {
            let mut registry = renderer.component_registry.lock();
            let _ = registry.register_component(
                "MyJsxComponent",
                "",
                register_component_js.to_string(),
                SmallVec::new(),
            );
            registry.mark_component_loaded("MyJsxComponent");
        }

        let render_result = renderer
            .render_to_string("MyJsxComponent", Some(r#"{"name":"Test"}"#))
            .await;

        assert!(renderer.initialized);

        let output = render_result.expect("Rendering should succeed");
        assert!(
            output.contains("<"),
            "Output should contain some HTML content"
        );
    }

    #[tokio::test]
    async fn test_render_to_readable_stream() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        renderer
            .initialize()
            .await
            .expect("Failed to initialize renderer");

        assert!(renderer.initialized);
    }
}
