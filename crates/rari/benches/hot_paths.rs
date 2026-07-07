//! Per-request and build-time hot path benchmarks for the main `rari` crate.
//!
//! Covers app and API routing, HTML sanitization and metadata injection, response
//! compression, cache keys and memory cache, flight protocol sorting, directive
//! scans, component ID generation, TS/TSX transpilation, and server-action arg
//! validation.

#![expect(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, OnceLock};

use bytes::Bytes;
use deno_core::{ModuleCodeString, ModuleName};
use divan::{Bencher, black_box};
use rari::{
    rendering::{
        base::sanitizer::sanitize_html_output,
        layout::{
            LayoutRenderContext, create_component_id, create_layout_context, generate_cache_key,
            sort_flight_protocol,
        },
    },
    runtime::transpile::maybe_transpile_source,
    server::{
        actions::{ValidationConfig, validate_and_sanitize_args},
        cache::{CacheHandler, bench_memory_cache_handler, response::ResponseCache},
        compression::{CompressionEncoding, compress_body},
        core::utils::component::{
            extract_component_id, has_use_client_directive, has_use_server_directive,
            readable_component_id, short_hash,
        },
        rendering::metadata_injection::{bench_page_metadata, inject_metadata},
        routing::{
            AppRouteMatch, AppRouter,
            api_routes::{bench_api_route_manifest, bench_match_api_route},
            app_router::bench_route_manifest,
        },
    },
};
use rustc_hash::FxHashMap;
use serde_json::json;
use tokio::runtime::{Builder, Runtime};

fn main() {
    divan::main();
}

static TOKIO: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    TOKIO.get_or_init(|| Builder::new_current_thread().enable_all().build().expect("tokio runtime"))
}

fn layout_context(route_match: &AppRouteMatch) -> LayoutRenderContext {
    let mut search_params = FxHashMap::default();
    search_params.insert("q".to_string(), vec!["rari".to_string(), "bench".to_string()]);
    search_params.insert("page".to_string(), vec!["2".to_string()]);

    create_layout_context(
        route_match.params.clone(),
        search_params,
        FxHashMap::default(),
        route_match.pathname.clone(),
    )
}

const HTML_SHELL: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>Benchmark Page</title></head>
<body>
<div id="root"><main><h1>Welcome</h1><p>Lorem ipsum dolor sit amet.</p></main></div>
</body>
</html>
"#;

fn html_page_shell() -> Bytes {
    let mut html = String::with_capacity(64 * 1024);
    for _ in 0..200 {
        html.push_str(HTML_SHELL);
    }
    Bytes::from(html)
}

const SANITIZE_CLEAN_HTML: &str = r"<div><h1>Title</h1><p>Paragraph with normal content.</p></div>";

const SANITIZE_LEAKY_HTML: &str = r#"<div>Start<pre>\{"debug": "info"\}</pre>Middle\{"id": "456"\}End<div>[{"id":"1"},{"id":"2"}]</div></div>"#;

const TSX_MODULE: &str = r#"
import type { ReactNode } from 'react'

export interface CardProps {
  title: string
  children: ReactNode
}

export function Card({ title, children }: CardProps) {
  return (
    <section>
      <h2>{title}</h2>
      {children}
    </section>
  )
}

export default function Page() {
  return <Card title="Home">Hello</Card>
}
"#;

const PLAIN_COMPONENT: &str = r"
import { useState } from 'react'

export function Counter({ initial }: { initial: number }) {
  const [count, setCount] = useState(initial)
  return <button onClick={() => setCount((c) => c + 1)}>{count}</button>
}

export default function Page() {
  return <Counter initial={0} />
}
";

const USE_CLIENT_COMPONENT: &str = r"'use client'

import { useState } from 'react'

export function Counter() {
  const [count, setCount] = useState(0)
  return <button onClick={() => setCount((c) => c + 1)}>{count}</button>
}
";

const USE_SERVER_MODULE: &str = r"'use server'

export async function createUser(name: string) {
  return { id: '1', name }
}
";

const METADATA_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Default Title</title>
    <meta name="description" content="placeholder">
</head>
<body><div id="root"></div></body>
</html>"#;

const FLIGHT_PROTOCOL: &str = "a:\"tail\"\n3:\"c\"\n1:\"a\"\n5:\"e\"\n2:\"b\"\n4:\"d\"\n";

fn action_args() -> Vec<serde_json::Value> {
    vec![json!({
        "user": {
            "__proto__": { "isAdmin": true },
            "name": "Jane",
            "settings": {
                "constructor": "bad",
                "theme": "dark",
                "tags": ["a", "b", "c"]
            }
        },
        "items": (0..20).map(|i| json!({ "id": i, "label": format!("item-{i}") })).collect::<Vec<_>>()
    })]
}

#[divan::bench]
fn match_route_static(bencher: Bencher) {
    let router = AppRouter::new(bench_route_manifest());
    bencher.bench(|| black_box(router.match_route("/about")).unwrap());
}

#[divan::bench]
fn match_route_dynamic(bencher: Bencher) {
    let router = AppRouter::new(bench_route_manifest());
    bencher.bench(|| black_box(router.match_route("/blog/hello-world")).unwrap());
}

#[divan::bench]
fn match_route_catch_all(bencher: Bencher) {
    let router = AppRouter::new(bench_route_manifest());
    bencher.bench(|| black_box(router.match_route("/docs/getting-started/installation")).unwrap());
}

#[divan::bench]
fn match_route_miss(bencher: Bencher) {
    let router = AppRouter::new(bench_route_manifest());
    bencher.bench(|| black_box(router.match_route("/missing-route")).is_err());
}

#[divan::bench]
fn sanitize_html_clean(bencher: Bencher) {
    bencher.bench(|| sanitize_html_output(black_box(SANITIZE_CLEAN_HTML)));
}

#[divan::bench]
fn sanitize_html_leaky(bencher: Bencher) {
    bencher.bench(|| sanitize_html_output(black_box(SANITIZE_LEAKY_HTML)));
}

#[divan::bench]
fn compress_body_gzip(bencher: Bencher) {
    let body = html_page_shell();
    let rt = runtime();
    bencher.bench(|| {
        rt.block_on(async {
            black_box(compress_body(body.clone(), CompressionEncoding::Gzip).await)
        })
    });
}

#[divan::bench]
fn layout_generate_cache_key(bencher: Bencher) {
    let router = AppRouter::new(bench_route_manifest());
    let route_match = router.match_route("/blog/hello-world").unwrap();
    let context = layout_context(&route_match);
    bencher.bench(|| black_box(generate_cache_key(black_box(&route_match), black_box(&context))));
}

#[divan::bench]
fn response_generate_cache_key(bencher: Bencher) {
    let mut params = FxHashMap::default();
    params.insert("slug".to_string(), "hello-world".to_string());
    params.insert("lang".to_string(), "en".to_string());

    bencher.bench(|| {
        black_box(ResponseCache::generate_cache_key_with_mode(
            "/blog/[slug]",
            Some(black_box(&params)),
            Some("static"),
        ))
    });
}

#[divan::bench]
fn response_generate_etag(bencher: Bencher) {
    let body = html_page_shell();
    bencher.bench(|| black_box(ResponseCache::generate_etag(black_box(body.as_ref()))));
}

#[divan::bench]
fn maybe_transpile_tsx(bencher: Bencher) {
    let module_name: ModuleName = "/app/page.tsx".to_string().into();
    let source: ModuleCodeString = TSX_MODULE.to_string().into();
    bencher.bench(|| {
        black_box(
            maybe_transpile_source(black_box(&module_name), black_box(source.try_clone().unwrap()))
                .unwrap(),
        )
    });
}

#[divan::bench]
fn api_match_route_static(bencher: Bencher) {
    let manifest = bench_api_route_manifest();
    bencher.bench(|| black_box(bench_match_api_route(&manifest, "/api/health", "GET")).unwrap());
}

#[divan::bench]
fn api_match_route_dynamic(bencher: Bencher) {
    let manifest = bench_api_route_manifest();
    bencher.bench(|| black_box(bench_match_api_route(&manifest, "/api/users/42", "GET")).unwrap());
}

#[divan::bench]
fn api_match_route_catch_all(bencher: Bencher) {
    let manifest = bench_api_route_manifest();
    bencher.bench(|| {
        black_box(bench_match_api_route(&manifest, "/api/files/docs/guide.pdf", "GET")).unwrap()
    });
}

#[divan::bench]
fn api_match_route_miss(bencher: Bencher) {
    let manifest = bench_api_route_manifest();
    bencher.bench(|| black_box(bench_match_api_route(&manifest, "/api/missing", "GET")).is_err());
}

#[divan::bench]
fn sort_flight_protocol_rows(bencher: Bencher) {
    bencher.bench(|| black_box(sort_flight_protocol(black_box(FLIGHT_PROTOCOL))));
}

#[divan::bench]
fn inject_metadata_rich(bencher: Bencher) {
    let metadata = bench_page_metadata();
    bencher
        .bench(|| black_box(inject_metadata(black_box(METADATA_HTML), black_box(&metadata), None)));
}

#[divan::bench]
fn has_use_client_directive_hit(bencher: Bencher) {
    bencher.bench(|| black_box(has_use_client_directive(black_box(USE_CLIENT_COMPONENT))));
}

#[divan::bench]
fn has_use_client_directive_miss(bencher: Bencher) {
    bencher.bench(|| black_box(has_use_client_directive(black_box(PLAIN_COMPONENT))));
}

#[divan::bench]
fn has_use_server_directive_hit(bencher: Bencher) {
    bencher.bench(|| black_box(has_use_server_directive(black_box(USE_SERVER_MODULE))));
}

#[divan::bench]
fn has_use_server_directive_miss(bencher: Bencher) {
    bencher.bench(|| black_box(has_use_server_directive(black_box(PLAIN_COMPONENT))));
}

#[divan::bench]
fn short_hash_component_path(bencher: Bencher) {
    bencher.bench(|| black_box(short_hash(black_box("app/blog/[slug]/page.tsx"))));
}

#[divan::bench]
fn readable_component_id_path(bencher: Bencher) {
    bencher.bench(|| black_box(readable_component_id(black_box("app/blog/[slug]/page.tsx"))));
}

#[divan::bench]
fn create_component_id_path(bencher: Bencher) {
    bencher.bench(|| black_box(create_component_id(black_box("app/blog/[slug]/page.tsx"))));
}

#[divan::bench]
fn extract_component_id_path(bencher: Bencher) {
    bencher.bench(|| black_box(extract_component_id(black_box("src/app/blog/page.tsx")).unwrap()));
}

#[divan::bench]
fn memory_cache_get_hit(bencher: Bencher) {
    let handler = Arc::new(bench_memory_cache_handler());
    let rt = runtime();
    rt.block_on(async {
        handler.set("bench-key", vec![0u8; 4096], 3600).await.unwrap();
    });

    bencher.bench(|| {
        rt.block_on(async { black_box(handler.get(black_box("bench-key")).await.unwrap()) })
    });
}

#[divan::bench]
fn memory_cache_set(bencher: Bencher) {
    let handler = Arc::new(bench_memory_cache_handler());
    let rt = runtime();
    let value = vec![0u8; 4096];

    bencher.bench(|| {
        rt.block_on(async {
            black_box(
                handler
                    .set(black_box("bench-set-key"), black_box(value.clone()), black_box(3600))
                    .await
                    .unwrap(),
            )
        })
    });
}

#[divan::bench]
fn validate_and_sanitize_action_args(bencher: Bencher) {
    let config = ValidationConfig::production();
    let args = action_args();
    bencher.bench(|| {
        black_box(validate_and_sanitize_args(black_box(&args), black_box(&config)).unwrap())
    });
}
