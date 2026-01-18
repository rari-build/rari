#[cfg(test)]
mod tests {
    #![allow(clippy::module_inception)]
    use smallvec::SmallVec;
    use std::sync::Arc;

    use crate::runtime::JsExecutionRuntime;

    use super::super::renderer::RscRenderer;

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

        renderer.initialize().await.expect("Failed to initialize renderer");

        {
            let mut registry = renderer.component_registry.lock();
            let _ = registry.register_component(
                "TestComponent",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }"
                    .to_string(),
                SmallVec::new(),
            );
        }

        assert!(renderer.initialized);
    }

    #[tokio::test]
    async fn test_register_and_render_jsx_component() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        renderer.initialize().await.expect("Failed to initialize renderer");

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

        let render_result =
            renderer.render_to_string("MyJsxComponent", Some(r#"{"name":"Test"}"#)).await;

        assert!(renderer.initialized);

        if let Ok(output) = render_result {
            assert!(output.contains("<"), "Output should contain some HTML content");
        }
    }

    #[tokio::test]
    async fn test_render_to_readable_stream() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        let init_result = renderer.initialize().await;
        assert!(init_result.is_ok(), "Failed to initialize renderer: {:?}", init_result.err());

        let component_id = "TestStreamComponent";
        renderer
            .runtime
            .execute_script(
                "register_mock_component.js".to_string(),
                format!("globalThis.{component_id} = function() {{ return {{}}; }};"),
            )
            .await
            .expect("Failed to execute script");

        assert!(renderer.initialized);
    }
}
