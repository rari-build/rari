//! Benchmarks for the `use cache` source transformer.
//!
//! These cover the build-time hot paths of `rari_use_cache`:
//! - `detect_use_cache`: fast byte-scan pre-filter run on every source file
//! - `transform_source`: the full parse -> visit -> codegen pipeline
//! - `generate_reference_id`: the SHA-256 based stable reference id generator
//!
//! Fixtures mirror real app patterns: function-level directives, file-level
//! `'use cache'`, remote cache kinds, and closure capture for cache keys.

use divan::{Bencher, black_box};
use rari_use_cache::{directive, id, transform};

fn main() {
    divan::main();
}

// A component module that does NOT use the cache directive. This represents
// the common case where the pre-filter should bail out quickly.
const PLAIN_COMPONENT: &str = r"
import React from 'react'
import { useState } from 'react'

export function Counter({ initial }: { initial: number }) {
  const [count, setCount] = useState(initial)
  return (
    <button onClick={() => setCount((c) => c + 1)}>
      Count: {count}
    </button>
  )
}

export default function App() {
  return <Counter initial={0} />
}
";

// Function-level directives on async exports, including a non-default cache kind
// and closure capture (`PAGE_SIZE`) for bound-arg key material.
const CACHED_MODULE: &str = r"
import { db } from './db'
import { formatUser } from './utils'

const PAGE_SIZE = 20

export async function getUser(id: string) {
  'use cache'
  const user = await db.users.findById(id)
  return formatUser(user)
}

export async function listUsers(page: number) {
  'use cache: stale-while-revalidate'
  const offset = page * PAGE_SIZE
  const rows = await db.users.list(offset, PAGE_SIZE)
  return rows.map((row) => formatUser(row))
}

export default async function dashboard(userId: string) {
  'use cache'
  const user = await getUser(userId)
  const users = await listUsers(0)
  return { user, users }
}
";

// File-level directive: all async exports inherit the module cache scope.
const FILE_LEVEL_CACHED_MODULE: &str = r"
'use cache'

import { db } from './db'

const PAGE_SIZE = 20

export async function getCachedItems(label: string) {
  const rows = await db.items.list(PAGE_SIZE)
  return rows.map((row) => `${label}:${row.id}`)
}

export async function getCachedCount() {
  return db.items.count()
}
";

// Remote cache kind used for shared redis/redb storage backends.
const REMOTE_CACHED_MODULE: &str = r"
import { db } from './db'

let callCount = 0

export async function getRemoteUser(id: string) {
  'use cache: remote'
  callCount += 1
  const user = await db.users.findById(id)
  return { id: user.id, calls: callCount }
}

export async function listRemoteUsers() {
  'use cache: remote'
  return db.users.list(0, 10)
}
";

#[divan::bench]
fn detect_use_cache_hit(bencher: Bencher) {
    bencher.bench(|| directive::detect_use_cache(black_box(CACHED_MODULE)));
}

#[divan::bench]
fn detect_use_cache_file_level(bencher: Bencher) {
    bencher.bench(|| directive::detect_use_cache(black_box(FILE_LEVEL_CACHED_MODULE)));
}

#[divan::bench]
fn detect_use_cache_remote(bencher: Bencher) {
    bencher.bench(|| directive::detect_use_cache(black_box(REMOTE_CACHED_MODULE)));
}

#[divan::bench]
fn detect_use_cache_miss(bencher: Bencher) {
    bencher.bench(|| directive::detect_use_cache(black_box(PLAIN_COMPONENT)));
}

#[divan::bench]
fn transform_cached_module(bencher: Bencher) {
    bencher.bench(|| {
        transform::transform_source(
            black_box(CACHED_MODULE),
            black_box("dashboard.tsx"),
            black_box("rari-use-cache-v1"),
        )
    });
}

#[divan::bench]
fn transform_file_level_cached_module(bencher: Bencher) {
    bencher.bench(|| {
        transform::transform_source(
            black_box(FILE_LEVEL_CACHED_MODULE),
            black_box("cached-helpers.ts"),
            black_box("rari-use-cache-v1"),
        )
    });
}

#[divan::bench]
fn transform_remote_cached_module(bencher: Bencher) {
    bencher.bench(|| {
        transform::transform_source(
            black_box(REMOTE_CACHED_MODULE),
            black_box("use-cache-remote/page.tsx"),
            black_box("rari-use-cache-v1"),
        )
    });
}

#[divan::bench]
fn transform_plain_component(bencher: Bencher) {
    bencher.bench(|| {
        transform::transform_source(
            black_box(PLAIN_COMPONENT),
            black_box("app.tsx"),
            black_box("rari-use-cache-v1"),
        )
    });
}

#[divan::bench]
fn generate_reference_id(bencher: Bencher) {
    bencher.bench(|| {
        id::generate_reference_id(
            black_box("rari-use-cache-v1"),
            black_box("src/app/dashboard/page.tsx"),
            black_box("getUser"),
            black_box(true),
        )
    });
}

#[divan::bench]
fn generate_cache_export_name(bencher: Bencher) {
    bencher.bench(|| id::generate_cache_export_name(black_box(3), black_box("getUserProfile")));
}
