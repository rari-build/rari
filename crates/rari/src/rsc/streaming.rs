use crate::error::RariError;
use futures::StreamExt;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, channel};
use tracing::error;

const DEFAULT_CHANNEL_BUFFER_SIZE: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RscStreamChunkType {
    ModuleReference,
    Import,
    ReactElement,
    Symbol,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RscStreamChunkMetadata {
    pub chunk_type: RscStreamChunkType,
    pub row_id: String,
    pub is_final: bool,
    pub error: Option<RscStreamError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RscStreamError {
    pub message: String,
    pub stack: Option<String>,
    pub digest: Option<String>,
}

pub struct RscStream {
    receiver: Receiver<Result<Vec<u8>, String>>,
}

#[derive(Debug, Clone)]
pub struct RscStreamChunk {
    pub data: Vec<u8>,
    pub metadata: RscStreamChunkMetadata,
}

impl Stream for RscStream {
    type Item = Result<Vec<u8>, RariError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(Some(Ok(data))) => Poll::Ready(Some(Ok(data))),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(RariError::internal(err)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl RscStream {
    pub fn new(receiver: Receiver<Result<Vec<u8>, String>>) -> Self {
        Self { receiver }
    }

    pub fn create() -> (Self, tokio::sync::mpsc::Sender<Result<Vec<u8>, String>>) {
        let (sender, receiver) = channel(DEFAULT_CHANNEL_BUFFER_SIZE);
        (Self::new(receiver), sender)
    }

    pub fn add_chunk(&mut self, data: Vec<u8>) -> Result<(), RariError> {
        let (sender, new_receiver) = channel(DEFAULT_CHANNEL_BUFFER_SIZE);

        sender
            .try_send(Ok(data))
            .map_err(|e| RariError::internal(format!("Failed to send chunk: {e}")))?;

        self.receiver = new_receiver;

        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), RariError> {
        Ok(())
    }

    pub fn process_raw_chunk(raw_data: &[u8]) -> Result<RscStreamChunk, RariError> {
        let raw_str = String::from_utf8(raw_data.to_vec())
            .map_err(|e| RariError::serialization(format!("Invalid UTF-8: {e}")))?;

        let lines: Vec<&str> = raw_str.lines().collect();
        if lines.is_empty() {
            return Err(RariError::serialization("Empty RSC payload".to_string()));
        }

        let line = lines[0];

        let colon_pos = line.find(':').ok_or_else(|| {
            RariError::serialization(format!("Invalid RSC line format, missing colon: {line}"))
        })?;

        let row_id = &line[..colon_pos];
        let rest = &line[colon_pos + 1..];

        let (chunk_type, row_data) = if let Some(stripped) = rest.strip_prefix('M') {
            (RscStreamChunkType::ModuleReference, stripped)
        } else if let Some(stripped) = rest.strip_prefix('I') {
            (RscStreamChunkType::Import, stripped)
        } else if let Some(stripped) = rest.strip_prefix('J') {
            (RscStreamChunkType::ReactElement, stripped)
        } else if let Some(stripped) = rest.strip_prefix('S') {
            (RscStreamChunkType::Symbol, stripped)
        } else if let Some(stripped) = rest.strip_prefix('E') {
            (RscStreamChunkType::Error, stripped)
        } else {
            (RscStreamChunkType::ReactElement, rest)
        };

        let error = if chunk_type == RscStreamChunkType::Error {
            let error_data: serde_json::Value = serde_json::from_str(row_data)
                .map_err(|e| RariError::serialization(format!("Invalid error JSON: {e}")))?;

            Some(RscStreamError {
                message: error_data
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error")
                    .to_string(),
                stack: error_data.get("stack").and_then(|s| s.as_str()).map(String::from),
                digest: error_data.get("digest").and_then(|d| d.as_str()).map(String::from),
            })
        } else {
            if !row_data.is_empty() && chunk_type != RscStreamChunkType::Symbol {
                serde_json::from_str::<serde_json::Value>(row_data).map_err(|e| {
                    RariError::serialization(format!("Invalid JSON in row data: {e}"))
                })?;
            }
            None
        };

        Ok(RscStreamChunk {
            data: raw_data.to_vec(),
            metadata: RscStreamChunkMetadata {
                chunk_type,
                row_id: row_id.to_string(),
                is_final: false,
                error,
            },
        })
    }

    pub fn process_multi_row_chunk(raw_data: &[u8]) -> Result<Vec<RscStreamChunk>, RariError> {
        let raw_str = String::from_utf8(raw_data.to_vec())
            .map_err(|e| RariError::serialization(format!("Invalid UTF-8: {e}")))?;

        let mut chunks = Vec::new();
        for line in raw_str.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let chunk = Self::process_raw_chunk(line.as_bytes())?;
            chunks.push(chunk);
        }

        if chunks.is_empty() {
            return Err(RariError::serialization("No valid RSC rows found".to_string()));
        }

        Ok(chunks)
    }

    pub fn format_as_rsc_row(row_id: &str, row_tag: &str, data: &str) -> String {
        match row_tag {
            "M" | "I" => format!("{row_id}:{row_tag}[{data}]"),
            "J" => format!("{row_id}:{data}"),
            "S" => format!("{row_id}:{data}"),
            "E" => format!("{row_id}:E{{{data}}}"),
            _ => format!("{row_id}:{data}"),
        }
    }

    #[allow(clippy::disallowed_methods)]
    pub fn create_module_row(row_id: &str, module_id: &str, chunks: &[&str], name: &str) -> String {
        let chunks_json = serde_json::to_string(&chunks).unwrap_or_else(|_| "[]".to_string());
        let module_data = if name.is_empty() {
            format!("[\"{module_id}\",{chunks_json},\"\"]")
        } else {
            format!("[\"{module_id}\",{chunks_json},\"{name}\"]")
        };
        Self::format_as_rsc_row(row_id, "I", &module_data)
    }

    pub fn create_element_row(row_id: &str, element_data: &serde_json::Value) -> String {
        Self::format_as_rsc_row(row_id, "", &element_data.to_string())
    }

    pub fn create_symbol_row(row_id: &str, symbol_ref: &str) -> String {
        Self::format_as_rsc_row(row_id, "S", &format!("Symbol.for(\"{symbol_ref}\")"))
    }

    #[allow(clippy::disallowed_methods)]
    pub fn create_error_row(row_id: &str, error: &RscStreamError) -> String {
        let error_data = serde_json::json!({
            "digest": error.digest.as_ref().unwrap_or(&"".to_string()),
            "message": error.message,
            "stack": error.stack
        });
        Self::format_as_rsc_row(row_id, "E", &error_data.to_string())
    }
}

pub trait RscStreamingExt {
    fn render_to_stream(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError>;

    fn render_to_writer<W: Write + Send + 'static>(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<(), RariError>;

    fn render_to_enhanced_stream(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscEnhancedStream, RariError>;

    fn render_with_readable_stream(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError>;

    fn invoke_server_component(
        &mut self,
        server_reference_id: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, RariError>;
}

pub struct RscEnhancedStream {
    stream: RscStream,
    shell_complete: bool,
    chunks_received: usize,
    complete: bool,
    error: Option<RscStreamError>,
    module_count: usize,
    element_count: usize,
}

impl RscEnhancedStream {
    pub fn new(stream: RscStream) -> Self {
        Self {
            stream,
            shell_complete: false,
            chunks_received: 0,
            complete: false,
            error: None,
            module_count: 0,
            element_count: 0,
        }
    }

    pub fn is_shell_complete(&self) -> bool {
        self.shell_complete
    }

    pub fn chunks_received(&self) -> usize {
        self.chunks_received
    }

    pub fn module_count(&self) -> usize {
        self.module_count
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn error(&self) -> Option<&RscStreamError> {
        self.error.as_ref()
    }

    pub async fn wait_for_shell(&mut self) -> Result<(), RariError> {
        if self.shell_complete {
            return Ok(());
        }

        while let Some(chunk_result) = self.stream.next().await {
            let chunk = chunk_result?;
            let processed_chunk = RscStream::process_raw_chunk(&chunk)?;
            self.chunks_received += 1;

            match processed_chunk.metadata.chunk_type {
                RscStreamChunkType::ReactElement => {
                    if !self.shell_complete {
                        self.shell_complete = true;
                        return Ok(());
                    }
                }
                RscStreamChunkType::Error => {
                    self.error = processed_chunk.metadata.error;
                    return Err(RariError::internal("Stream error encountered"));
                }
                _ => {}
            }

            if processed_chunk.metadata.is_final {
                self.complete = true;
                return Ok(());
            }
        }
        Err(RariError::internal("Stream ended unexpectedly while waiting for shell"))
    }

    pub async fn collect_html(&mut self) -> Result<String, RariError> {
        let mut html_parts = SmallVec::<[String; 4]>::new();

        while let Some(chunk_result) = self.stream.next().await {
            let chunk_bytes = chunk_result?;
            let processed_chunk = RscStream::process_raw_chunk(&chunk_bytes)?;

            match processed_chunk.metadata.chunk_type {
                RscStreamChunkType::ReactElement => {
                    let chunk_str = String::from_utf8(processed_chunk.data).map_err(|e| {
                        RariError::Serialization(format!("Non-UTF8 data: {e}"), None)
                    })?;
                    html_parts.push(chunk_str);
                }
                RscStreamChunkType::Error => {
                    self.error = processed_chunk.metadata.error;
                    let error_message =
                        self.error.as_ref().map(|e| e.message.clone()).unwrap_or_else(|| {
                            "Unknown error occurred during streaming".to_string()
                        });
                    return Err(RariError::Internal(error_message, None));
                }
                _ => {}
            }

            if processed_chunk.metadata.is_final {
                self.complete = true;
                break;
            }
        }

        if !self.complete {
            return Err(RariError::internal("Stream did not complete"));
        }
        Ok(html_parts.join(""))
    }
}

impl Stream for RscEnhancedStream {
    type Item = Result<RscStreamChunk, RariError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.complete {
            return Poll::Ready(None);
        }
        match Pin::new(&mut this.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(raw_chunk_bytes))) => {
                this.chunks_received += 1;
                match RscStream::process_raw_chunk(&raw_chunk_bytes) {
                    Ok(processed_chunk) => {
                        match processed_chunk.metadata.chunk_type {
                            RscStreamChunkType::ModuleReference => {
                                this.module_count += 1;
                            }
                            RscStreamChunkType::ReactElement => {
                                this.element_count += 1;
                                if !this.shell_complete {
                                    this.shell_complete = true;
                                }
                            }
                            RscStreamChunkType::Error => {
                                this.error = processed_chunk.metadata.error.clone();
                            }
                            _ => {}
                        }

                        if processed_chunk.metadata.is_final {
                            this.complete = true;
                        }
                        Poll::Ready(Some(Ok(processed_chunk)))
                    }
                    Err(e) => {
                        this.complete = true;
                        this.error = Some(RscStreamError {
                            message: e.to_string(),
                            stack: None,
                            digest: None,
                        });
                        Poll::Ready(Some(Err(e)))
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => {
                this.complete = true;
                this.error =
                    Some(RscStreamError { message: e.to_string(), stack: None, digest: None });
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                this.complete = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl RscStreamingExt for super::renderer::RscRenderer {
    fn render_to_stream(
        &mut self,
        _component_id: &str,
        _props: Option<&str>,
    ) -> Result<RscStream, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }
        let (_sender, receiver) = channel(DEFAULT_CHANNEL_BUFFER_SIZE);
        Ok(RscStream { receiver })
    }

    #[allow(clippy::disallowed_methods)]
    fn render_to_writer<W: Write + Send + 'static>(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<(), RariError> {
        if !self.initialized {
            let err_msg = "RSC renderer not initialized".to_string();
            return Err(RariError::internal(err_msg));
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| RariError::internal(format!("Failed to create Tokio runtime: {e}")))?;

        let component_id_string = component_id.to_string();
        let props_string = props.map(|p| p.to_string());

        let setup_result = runtime.block_on(async {
            let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, String>>(32);

            let mut row_counter = 0u32;

            let module_row = RscStream::create_module_row(
                &row_counter.to_string(),
                &format!("rsc-component-{component_id_string}"),
                &["main"],
                "default",
            );
            row_counter += 1;

            if sender.send(Ok(format!("{module_row}\n").into_bytes())).await.is_err() {
                error!("Failed to send module row");
            }

            let element_data = serde_json::json!([
                "$",
                format!("${}", 0),
                null,
                {
                    "component": component_id_string,
                    "props": props_string.unwrap_or_else(|| "{}".to_string())
                }
            ]);

            let element_row =
                RscStream::create_element_row(&row_counter.to_string(), &element_data);

            if sender.send(Ok(format!("{element_row}\n").into_bytes())).await.is_err() {
                error!("Failed to send element row");
            }

            Ok::<_, RariError>(receiver)
        });

        let mut receiver = match setup_result {
            Ok(r) => r,
            Err(e) => {
                let _err_msg = format!("Failed to set up streaming: {e}");
                return Err(e);
            }
        };

        tokio::spawn(async move {
            while let Some(message_result) = receiver.recv().await {
                match message_result {
                    Ok(_chunk_bytes) => {}
                    Err(error_string) => {
                        let err_msg = format!("Error during RSC stream processing: {error_string}");
                        error!("{err_msg}");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    #[allow(clippy::disallowed_methods)]
    fn render_to_enhanced_stream(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscEnhancedStream, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, String>>(32);
        let component_id_string = component_id.to_string();
        let props_string = props.map(|p| p.to_string());

        let mut row_counter = 0u32;

        let module_row = RscStream::create_module_row(
            &row_counter.to_string(),
            &format!("rsc-component-{component_id_string}"),
            &["main"],
            "default",
        );
        row_counter += 1;

        if sender.try_send(Ok(format!("{module_row}\n").into_bytes())).is_err() {
            error!("Failed to send module row to enhanced stream");
        }

        let element_data = serde_json::json!([
            "$",
            format!("${}", 0),
            null,
            {
                "component": component_id_string,
                "props": props_string.unwrap_or_else(|| "{}".to_string())
            }
        ]);

        let element_row = RscStream::create_element_row(&row_counter.to_string(), &element_data);

        if sender.try_send(Ok(format!("{element_row}\n").into_bytes())).is_err() {
            error!("Failed to send element row to enhanced stream");
        }

        let stream = RscStream::new(receiver);
        Ok(RscEnhancedStream::new(stream))
    }

    #[allow(clippy::disallowed_methods)]
    fn render_with_readable_stream(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, String>>(64);
        let component_id_string = component_id.to_string();
        let props_string = props.map(|p| p.to_string());

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut row_counter = 0u32;

                let module_row = RscStream::create_module_row(
                    &row_counter.to_string(),
                    &format!("rsc-component-{component_id_string}"),
                    &["main"],
                    "default",
                );
                row_counter += 1;

                if sender.send(Ok(format!("{module_row}\n").into_bytes())).await.is_err() {
                    error!("Failed to send module row");
                    return;
                }

                let element_data = serde_json::json!([
                    "$",
                    format!("${}", 0),
                    null,
                    {
                        "component": component_id_string,
                        "props": props_string.unwrap_or_else(|| "{}".to_string())
                    }
                ]);

                let element_row =
                    RscStream::create_element_row(&row_counter.to_string(), &element_data);

                if sender.send(Ok(format!("{element_row}\n").into_bytes())).await.is_err() {
                    error!("Failed to send element row");
                    return;
                }

                if component_id_string.contains("suspense") || component_id_string.contains("async")
                {
                    let suspense_row =
                        RscStream::create_symbol_row(&row_counter.to_string(), "$Sreact.suspense");
                    row_counter += 1;

                    if sender.send(Ok(format!("{suspense_row}\n").into_bytes())).await.is_err() {
                        error!("Failed to send suspense row");
                        return;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                    let resolved_data = serde_json::json!([
                        "$",
                        format!("${}", row_counter - 1),
                        null,
                        {
                            "fallback": ["$", "div", null, {"children": "Loading..."}],
                            "children": format!("Resolved content for {}", component_id_string)
                        }
                    ]);

                    let resolved_row =
                        RscStream::create_element_row(&row_counter.to_string(), &resolved_data);

                    if sender.send(Ok(format!("{resolved_row}\n").into_bytes())).await.is_err() {
                        error!("Failed to send resolved suspense content");
                    }
                }
            });
        } else {
            let mut row_counter = 0u32;

            let module_row = RscStream::create_module_row(
                &row_counter.to_string(),
                &format!("rsc-component-{component_id_string}"),
                &["main"],
                "default",
            );
            row_counter += 1;

            if sender.try_send(Ok(format!("{module_row}\n").into_bytes())).is_err() {
                error!("Failed to send module row");
            }

            let element_data = serde_json::json!([
                "$",
                format!("${}", 0),
                null,
                {
                    "component": component_id_string,
                    "props": props_string.unwrap_or_else(|| "{}".to_string())
                }
            ]);

            let element_row =
                RscStream::create_element_row(&row_counter.to_string(), &element_data);

            if sender.try_send(Ok(format!("{element_row}\n").into_bytes())).is_err() {
                error!("Failed to send element row");
            }
        }

        Ok(RscStream::new(receiver))
    }

    #[allow(clippy::disallowed_methods)]
    fn invoke_server_component(
        &mut self,
        server_reference_id: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        let result = serde_json::json!({
            "type": "server-component",
            "id": server_reference_id,
            "args": args,
            "result": {
                "element": ["$", "div", null, {
                    "data-server-component": server_reference_id,
                    "children": format!("Server component {} executed", server_reference_id)
                }]
            }
        });

        Ok(result)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::rsc::renderer::RscRenderer;
    use crate::runtime::JsExecutionRuntime;
    use futures::stream::StreamExt;
    use std::sync::Arc;

    type ChunkList = SmallVec<[Vec<u8>; 4]>;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_rsc_wire_format_parsing() {
        let test_payload = b"0:I[\"test-component\",[\"main\"],\"default\"]\n1:[\"$\",\"$L0\",null,{\"component\":\"TestComponent\"}]";

        let chunks =
            RscStream::process_multi_row_chunk(test_payload).expect("Failed to parse RSC payload");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].metadata.chunk_type, RscStreamChunkType::Import);
        assert_eq!(chunks[0].metadata.row_id, "0");
        assert_eq!(chunks[1].metadata.chunk_type, RscStreamChunkType::ReactElement);
        assert_eq!(chunks[1].metadata.row_id, "1");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_rsc_format_generation() {
        let module_row = RscStream::create_module_row("0", "test-component", &["main"], "default");
        assert!(module_row.starts_with("0:I["));
        assert!(module_row.contains("test-component"));

        let element_data = serde_json::json!(["$", "$0", null, {"test": "data"}]);
        let element_row = RscStream::create_element_row("1", &element_data);
        assert!(element_row.starts_with("1:["));

        let symbol_row = RscStream::create_symbol_row("2", "react.suspense");
        assert!(symbol_row.starts_with("2:Symbol.for("));
        assert!(symbol_row.contains("react.suspense"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_enhanced_stream_with_rsc_format() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime.clone());

        renderer.initialize().await.expect("Failed to initialize renderer");

        let mut enhanced_stream = renderer
            .render_to_enhanced_stream("TestComponent", Some("{\"test\":\"props\"}"))
            .expect("Failed to create enhanced stream");

        let mut received_chunks = ChunkList::new();
        let mut module_refs = 0;
        let mut elements = 0;

        while let Some(chunk_result) = enhanced_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    match chunk.metadata.chunk_type {
                        RscStreamChunkType::ModuleReference => module_refs += 1,
                        RscStreamChunkType::ReactElement => elements += 1,
                        _ => {}
                    }
                    received_chunks.push(chunk.data);
                }
                Err(_e) => {
                    break;
                }
            }

            if received_chunks.len() >= 2 {
                break;
            }
        }

        assert_eq!(enhanced_stream.module_count(), module_refs);
        assert_eq!(enhanced_stream.element_count(), elements);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_suspense_streaming() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime.clone());

        renderer.initialize().await.expect("Failed to initialize renderer");

        let stream = renderer
            .render_with_readable_stream("suspense-component", None)
            .expect("Failed to create readable stream");

        let mut enhanced_stream = RscEnhancedStream::new(stream);

        enhanced_stream.wait_for_shell().await.expect("Failed to wait for shell");

        assert!(enhanced_stream.is_shell_complete(), "Shell should be complete");
        assert!(enhanced_stream.chunks_received() > 0, "Should have received chunks");
    }

    #[test]
    fn test_error_row_creation() {
        let error = RscStreamError {
            message: "Test error".to_string(),
            stack: Some("Stack trace".to_string()),
            digest: Some("abc123".to_string()),
        };

        let error_row = RscStream::create_error_row("0", &error);
        assert!(error_row.starts_with("0:E"));
        assert!(error_row.contains("Test error"));
        assert!(error_row.contains("Stack trace"));
        assert!(error_row.contains("abc123"));
    }

    #[test]
    fn test_rsc_wire_format_compliance() {
        let module_row = RscStream::create_module_row(
            "1",
            "app/page.js",
            &["app", "static/chunks/app.js"],
            "default",
        );
        assert!(module_row.starts_with("1:I["));
        assert!(module_row.contains("\"app/page.js\""));
        assert!(module_row.contains("\"default\""));

        let element_data = serde_json::json!(["$", "div", null, {"children": "Hello"}]);
        let element_row = RscStream::create_element_row("2", &element_data);
        assert!(element_row.starts_with("2:["));
        assert!(element_row.starts_with("2:["));

        let symbol_row = RscStream::create_symbol_row("3", "react.element");
        assert!(symbol_row.starts_with("3:Symbol.for("));

        let error = RscStreamError {
            message: "Test error".to_string(),
            stack: Some("Stack trace".to_string()),
            digest: Some("abc123".to_string()),
        };
        let error_row = RscStream::create_error_row("4", &error);
        assert!(error_row.starts_with("4:E{"));
        assert!(error_row.contains("\"digest\":\"abc123\""));
        assert!(error_row.contains("\"message\":\"Test error\""));
    }

    #[test]
    fn test_metadata_compliance() {
        let metadata = RscStreamChunkMetadata {
            chunk_type: RscStreamChunkType::ReactElement,
            row_id: "1".to_string(),
            is_final: false,
            error: None,
        };

        assert_eq!(metadata.row_id, "1");
        assert!(!metadata.is_final);
        assert!(metadata.error.is_none());

        let error = RscStreamError {
            message: "Error message".to_string(),
            stack: Some("Stack".to_string()),
            digest: Some("digest123".to_string()),
        };

        assert_eq!(error.message, "Error message");
        assert_eq!(error.stack, Some("Stack".to_string()));
        assert_eq!(error.digest, Some("digest123".to_string()));
    }

    #[tokio::test]
    async fn test_streaming_compliance() {
        let (mut stream, sender) = RscStream::create();

        let payload = "1:I[\"app/page.js\",[\"app\",\"static/chunks/app.js\"],\"default\"]\n";
        sender.send(Ok(payload.as_bytes().to_vec())).await.unwrap();
        drop(sender);

        if let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.unwrap();
            let processed = RscStream::process_raw_chunk(&chunk).unwrap();

            assert_eq!(processed.metadata.chunk_type, RscStreamChunkType::Import);
            assert_eq!(processed.metadata.row_id, "1");
            assert!(!processed.metadata.is_final);
        }
    }

    #[tokio::test]
    async fn test_rsc_compliance() {
        let module_row = RscStream::create_module_row(
            "0",
            "app/page.js",
            &["app", "static/chunks/app.js"],
            "default",
        );

        assert!(module_row.starts_with("0:I["));
        assert!(module_row.contains("\"app/page.js\""));
        assert!(module_row.contains("[\"app\",\"static/chunks/app.js\"]"));
        assert!(module_row.contains("\"default\""));
        assert!(module_row.ends_with("]"));

        let element_data = serde_json::json!(["$", "div", null, {"children": "Hello World"}]);
        let element_row = RscStream::create_element_row("1", &element_data);

        assert!(element_row.starts_with("1:["));
        assert!(element_row.contains("\"$\""));
        assert!(element_row.contains("\"div\""));
        assert!(element_row.contains("\"Hello World\""));
        assert!(element_row.ends_with("]"));

        let error = RscStreamError {
            message: "Component failed to render".to_string(),
            stack: Some("Error: Component failed\n    at Component.render".to_string()),
            digest: Some("error-123".to_string()),
        };
        let error_row = RscStream::create_error_row("2", &error);

        assert!(error_row.starts_with("2:E{"));
        assert!(error_row.contains("\"message\":\"Component failed to render\""));
        assert!(error_row.contains("\"digest\":\"error-123\""));
        assert!(error_row.ends_with("}"));

        let rsc_payload = concat!(
            "0:I[\"app/page.js\",[\"app\",\"static/chunks/app.js\"],\"default\"]\n",
            "1:[\"$\",\"$L0\",null,{\"children\":\"Hello from React Server Components\"}]\n"
        );

        let chunks = RscStream::process_multi_row_chunk(rsc_payload.as_bytes())
            .expect("Should parse valid RSC payload");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].metadata.chunk_type, RscStreamChunkType::Import);
        assert_eq!(chunks[1].metadata.chunk_type, RscStreamChunkType::ReactElement);
    }

    #[tokio::test]
    async fn test_streaming_with_react_server_dom_format() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime.clone());

        renderer.initialize().await.expect("Failed to initialize renderer");

        let mut stream = renderer
            .render_with_readable_stream("TestComponent", Some("{\"message\":\"Hello\"}"))
            .expect("Failed to create readable stream");

        let mut received_data = Vec::new();
        let mut stream_pin = std::pin::Pin::new(&mut stream);

        for _ in 0..3 {
            if let Some(chunk_result) = futures::StreamExt::next(&mut stream_pin).await {
                match chunk_result {
                    Ok(chunk_bytes) => {
                        let chunk_str = String::from_utf8_lossy(&chunk_bytes);
                        received_data.push(chunk_str.to_string());

                        for line in chunk_str.lines() {
                            if !line.is_empty() {
                                assert!(
                                    line.contains(":"),
                                    "Each line should have row_id:data format"
                                );
                                let parts: Vec<&str> = line.splitn(2, ':').collect();
                                assert_eq!(
                                    parts.len(),
                                    2,
                                    "Should have exactly one colon separator"
                                );

                                assert!(
                                    parts[0].parse::<u32>().is_ok(),
                                    "Row ID should be numeric"
                                );

                                let data = parts[1];
                                assert!(
                                    data.starts_with("I[")
                                        || data.starts_with("[")
                                        || data.starts_with("E{")
                                        || data.starts_with("Symbol.for("),
                                    "Data should follow React RSC format, got: {data}"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        panic!("Stream error: {e}");
                    }
                }
            } else {
                break;
            }
        }

        assert!(!received_data.is_empty(), "Should have received some data");
    }

    #[tokio::test]
    async fn test_multi_chunk_streaming_complex() {
        let complex_payload = concat!(
            "0:I[\"app/components/Header.js\",[\"app\",\"static/chunks/header.js\"],\"default\"]\n",
            "1:I[\"app/components/Sidebar.js\",[\"app\",\"static/chunks/sidebar.js\"],\"default\"]\n",
            "2:I[\"app/components/Footer.js\",[\"app\",\"static/chunks/footer.js\"],\"default\"]\n",
            "3:I[\"app/lib/utils.js\",[\"app\",\"static/chunks/utils.js\"],\"formatDate\"]\n",
            "4:[\"$\",\"$L0\",null,{\"title\":\"My App\",\"user\":\"John\"}]\n",
            "5:[\"$\",\"div\",null,{\"className\":\"layout\",\"children\":[[\"$\",\"$L0\",\"header\",{}],[\"$\",\"main\",null,{\"children\":[\"$\",\"$L1\",\"sidebar\",{}]}],[\"$\",\"$L2\",\"footer\",{}]]}]\n",
            "6:[\"$\",\"$L1\",null,{\"navigation\":[\"Home\",\"About\",\"Contact\"]}]\n",
            "7:Symbol.for(\"react.suspense\")\n",
            "8:[\"$\",\"$L7\",null,{\"fallback\":[\"$\",\"div\",null,{\"children\":\"Loading...\"}],\"children\":[\"$\",\"div\",null,{\"children\":\"Async content loaded!\"}]}]\n"
        );

        let chunks = RscStream::process_multi_row_chunk(complex_payload.as_bytes())
            .expect("Should parse complex multi-chunk payload");

        assert_eq!(chunks.len(), 9, "Should parse all 9 chunks");

        let import_chunks: Vec<_> =
            chunks.iter().filter(|c| c.metadata.chunk_type == RscStreamChunkType::Import).collect();
        assert_eq!(import_chunks.len(), 4, "Should have 4 import chunks");

        let element_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.metadata.chunk_type == RscStreamChunkType::ReactElement)
            .collect();
        assert_eq!(element_chunks.len(), 4, "Should have 4 element chunks");

        let symbol_chunks: Vec<_> =
            chunks.iter().filter(|c| c.metadata.chunk_type == RscStreamChunkType::Symbol).collect();
        assert_eq!(symbol_chunks.len(), 1, "Should have 1 symbol chunk");

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.row_id, i.to_string(), "Row IDs should be sequential");
        }
    }

    #[tokio::test]
    async fn test_streaming_with_multiple_async_chunks() {
        let (mut stream, sender) = RscStream::create();

        let sender_task = tokio::spawn(async move {
            let module_chunk = "0:I[\"app/AsyncComponent.js\",[\"app\"],\"default\"]\n";
            sender.send(Ok(module_chunk.as_bytes().to_vec())).await.unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let element_chunk = "1:[\"$\",\"$L0\",null,{\"data\":\"chunk1\"}]\n";
            sender.send(Ok(element_chunk.as_bytes().to_vec())).await.unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let element_chunk2 = "2:[\"$\",\"div\",null,{\"children\":\"chunk2\"}]\n";
            sender.send(Ok(element_chunk2.as_bytes().to_vec())).await.unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let final_chunk = "3:[\"$\",\"span\",null,{\"children\":\"final\"}]\n";
            sender.send(Ok(final_chunk.as_bytes().to_vec())).await.unwrap();

            drop(sender);
        });

        let mut received_chunks = Vec::new();
        let mut chunk_types = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk_bytes) => {
                    let processed_chunk =
                        RscStream::process_raw_chunk(&chunk_bytes).expect("Should process chunk");
                    chunk_types.push(processed_chunk.metadata.chunk_type.clone());
                    received_chunks.push(chunk_bytes);
                }
                Err(e) => {
                    panic!("Stream error: {e}");
                }
            }
        }

        sender_task.await.unwrap();

        assert_eq!(received_chunks.len(), 4, "Should receive all 4 chunks");
        assert_eq!(chunk_types[0], RscStreamChunkType::Import);
        assert_eq!(chunk_types[1], RscStreamChunkType::ReactElement);
        assert_eq!(chunk_types[2], RscStreamChunkType::ReactElement);
        assert_eq!(chunk_types[3], RscStreamChunkType::ReactElement);
    }

    #[tokio::test]
    async fn test_enhanced_stream_multi_chunk_metrics() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime.clone());

        renderer.initialize().await.expect("Failed to initialize renderer");

        let mut enhanced_stream = renderer
            .render_to_enhanced_stream("MultiChunkComponent", Some("{\"complexity\":\"high\"}"))
            .expect("Failed to create enhanced stream");

        let mut total_chunks = 0;
        let mut max_modules = 0;
        let mut max_elements = 0;

        while let Some(chunk_result) = enhanced_stream.next().await {
            match chunk_result {
                Ok(_chunk) => {
                    total_chunks += 1;
                    max_modules = max_modules.max(enhanced_stream.module_count());
                    max_elements = max_elements.max(enhanced_stream.element_count());

                    if total_chunks >= 5 {
                        break;
                    }
                }
                Err(_e) => {
                    break;
                }
            }
        }

        assert!(total_chunks > 0, "Should receive multiple chunks");
        assert_eq!(enhanced_stream.chunks_received(), total_chunks, "Chunk count should match");

        println!("Processed {total_chunks} chunks, {max_modules} modules, {max_elements} elements");
    }

    #[tokio::test]
    async fn test_large_multi_chunk_payload() {
        let mut large_payload = String::new();
        let chunk_count = 50;

        for i in 0..chunk_count {
            if i % 2 == 0 {
                large_payload.push_str(&format!(
                    "{i}:I[\"app/component{i}.js\",[\"app\"],\"Component{i}\"]\n"
                ));
            } else {
                large_payload.push_str(&format!(
                    "{i}:[\"$\",\"div\",null,{{\"id\":\"element{i}\",\"children\":\"Content {i}\"}}]\n"
                ));
            }
        }

        let chunks = RscStream::process_multi_row_chunk(large_payload.as_bytes())
            .expect("Should parse large multi-chunk payload");

        assert_eq!(chunks.len(), chunk_count, "Should parse all {chunk_count} chunks");

        for (i, chunk) in chunks.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(chunk.metadata.chunk_type, RscStreamChunkType::Import);
            } else {
                assert_eq!(chunk.metadata.chunk_type, RscStreamChunkType::ReactElement);
            }
            assert_eq!(chunk.metadata.row_id, i.to_string());
        }
    }

    #[tokio::test]
    async fn test_multi_chunk_error_handling() {
        let payload_with_error = concat!(
            "0:I[\"app/GoodComponent.js\",[\"app\"],\"default\"]\n",
            "1:[\"$\",\"$L0\",null,{\"status\":\"ok\"}]\n",
            "2:E{\"message\":\"Component failed to render\",\"digest\":\"error123\",\"stack\":\"Error: Failed\\n    at Component.render\"}\n",
            "3:[\"$\",\"div\",null,{\"children\":\"This should still work\"}]\n"
        );

        let chunks = RscStream::process_multi_row_chunk(payload_with_error.as_bytes())
            .expect("Should parse payload with error");

        assert_eq!(chunks.len(), 4, "Should parse all chunks including error");

        let error_chunk = chunks
            .iter()
            .find(|c| c.metadata.chunk_type == RscStreamChunkType::Error)
            .expect("Should have error chunk");

        assert!(error_chunk.metadata.error.is_some(), "Error chunk should have error metadata");
        let error = error_chunk.metadata.error.as_ref().unwrap();
        assert_eq!(error.message, "Component failed to render");
        assert_eq!(error.digest.as_ref().unwrap(), "error123");

        let valid_chunks: Vec<_> =
            chunks.iter().filter(|c| c.metadata.chunk_type != RscStreamChunkType::Error).collect();
        assert_eq!(valid_chunks.len(), 3, "Should have 3 valid chunks");
    }
}
