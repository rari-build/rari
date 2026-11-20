// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use oxc::{
    semantic::SemanticBuilder,
    str::Ident,
    syntax::module_record::{ExportEntry, ExportExportName, ExportLocalName},
};
use serde::{Deserialize, Serialize};

use crate::ParsedSource;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleExportsAndReExports {
    pub exports: Vec<String>,
    pub reexports: Vec<String>,
}

impl ParsedSource<'_> {
    /// Analyzes the ES runtime exports for require ESM.
    ///
    /// The result describes the source as originally parsed and does not
    /// represent transpiled output.
    ///
    /// This is used during CJS export analysis when a CJS module
    /// re-exports an ESM module and the original CJS module needs
    /// to know the exports of the ESM module so it can create its
    /// wrapper ESM module.
    pub fn analyze_es_runtime_exports(&self) -> ModuleExportsAndReExports {
        fn is_local_type_only(entry: &ExportEntry<'_>, scoping: &oxc::semantic::Scoping) -> bool {
            if entry.module_request.is_some() {
                return false;
            }
            let ExportLocalName::Name(name) = &entry.local_name else {
                return false;
            };
            scoping
                .get_binding(scoping.root_scope_id(), Ident::from(name.name.as_str()))
                .is_some_and(|symbol| scoping.symbol_flags(symbol).is_type())
        }

        let module_record = self.module_record();
        let scoping = SemanticBuilder::new().build(self.program()).semantic.into_scoping();
        let mut result = ModuleExportsAndReExports::default();

        // Type-only exports (`export type { A }`, `export interface B {}`,
        // `export declare const c`) do not exist at runtime.
        let mut entries: Vec<&ExportEntry<'_>> = module_record
            .local_export_entries
            .iter()
            .chain(module_record.indirect_export_entries.iter())
            .filter(|entry| !entry.is_type && !is_local_type_only(entry, &scoping))
            .collect();
        // The module record groups entries by kind, so sort them back into
        // source order.
        entries.sort_by_key(|entry| entry.span.start);
        for entry in entries {
            match &entry.export_name {
                ExportExportName::Name(name) => {
                    result.exports.push(name.name.to_string());
                }
                ExportExportName::Default(_) => {
                    result.exports.push("default".to_string());
                }
                ExportExportName::Null => {}
            }
        }

        for entry in &module_record.star_export_entries {
            if entry.is_type {
                continue;
            }
            if let Some(module_request) = &entry.module_request {
                result.reexports.push(module_request.name.to_string());
            }
        }

        result
    }
}

#[cfg(test)]
#[expect(
    clippy::absolute_paths,
    clippy::unwrap_used,
    reason = "Ported code not yet updated to the workspace lint rules"
)]
mod test {
    use std::cell::RefCell;

    use deno_media_type::MediaType;
    use oxc::allocator::Allocator;

    use super::ModuleExportsAndReExports;
    use crate::{ModuleSpecifier, ParseParams, parse_module};

    struct Tester {
        analysis: RefCell<ModuleExportsAndReExports>,
    }

    impl Tester {
        pub fn assert_exports(&self, values: &[&str]) {
            let mut analysis = self.analysis.borrow_mut();
            assert_eq!(analysis.exports, values);
            analysis.exports.clear();
        }

        pub fn assert_reexports(&self, values: &[&str]) {
            let mut analysis = self.analysis.borrow_mut();
            assert_eq!(analysis.reexports, values);
            analysis.reexports.clear();
        }

        pub fn assert_empty(&self) {
            let analysis = self.analysis.borrow();
            assert!(analysis.exports.is_empty(), "Had exports: {}", analysis.exports.join(", "));
            assert!(
                analysis.reexports.is_empty(),
                "Had reexports: {}",
                analysis.reexports.join(", ")
            );
        }
    }

    impl Drop for Tester {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                self.assert_empty();
            }
        }
    }

    fn parse(source: &str) -> Tester {
        let allocator = Allocator::default();
        let parsed_source = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///example.ts").unwrap(),
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: true,
                scope_analysis: false,
                maybe_source_type: None,
            },
        )
        .unwrap();
        let analysis = parsed_source.analyze_es_runtime_exports();
        Tester { analysis: RefCell::new(analysis) }
    }

    #[test]
    fn runtime_exports_basic() {
        let tester = parse(
            "
export class A {}
export enum B {}
export module C.Test {}
export namespace C2.Test {}
export function d() {}
export const e = 1, f = 2;
export { g, h1 as h, other as 'testing-this' };
export * as y from './other.js';
export { z } from './other.js';
class Ignored1 {}
enum Ignored2 {}
module Ignored3 {}
namespace Ignored4 {}
function Ignored5() {}
const Ignored6 = 1;
",
        );
        tester.assert_exports(&[
            "A",
            "B",
            "C",
            "C2",
            "d",
            "e",
            "f",
            "g",
            "h",
            "testing-this",
            "y",
            "z",
        ]);
    }

    #[test]
    fn runtime_exports_default_named_export() {
        let tester = parse("export { a as default }");
        tester.assert_exports(&["default"]);
    }

    #[test]
    fn runtime_exports_source_order() {
        let tester = parse(
            "
export { z } from './other.js';
export const a = 1;
export * as y from './other.js';
export enum B {}
export namespace C {}
",
        );
        tester.assert_exports(&["z", "a", "y", "B", "C"]);
    }

    #[test]
    fn runtime_exports_destructuring() {
        let tester = parse(
            "
export const { a, b: [c, ...d], ...e } = obj;
export let [f, { g = 1 }] = arr;
",
        );
        tester.assert_exports(&["a", "c", "d", "e", "f", "g"]);
    }

    #[test]
    fn runtime_exports_ignores_types() {
        let tester = parse(
            "
export interface A {}
export type B = string;
export type { C };
export { type D, e } from './other.js';
export type { F };
export { type H, i };
export type * from './types.js';
export declare const f: number;
export const g = 1;
",
        );
        tester.assert_exports(&["e", "i", "g"]);
    }

    #[test]
    fn runtime_exports_ignores_locally_bound_type_only_exports() {
        let tester = parse(
            "
interface A {}
type B = string;
import type { C } from './types.js';
const value = 1;
export { A, B, C, value };
export { value as directValue } from './other.js';
",
        );
        tester.assert_exports(&["value", "directValue"]);
    }

    #[test]
    fn runtime_exports_default_expr() {
        let tester = parse("export default 5;");
        tester.assert_exports(&["default"]);
    }

    #[test]
    fn runtime_exports_default_decl() {
        let tester = parse("export default class A {}");
        tester.assert_exports(&["default"]);
        let tester = parse("export default function () {}");
        tester.assert_exports(&["default"]);
        let tester = parse("export { a as default };");
        tester.assert_exports(&["default"]);
    }

    #[test]
    fn runtime_re_export() {
        let tester = parse("export * from './other.js';");
        tester.assert_reexports(&["./other.js"]);
    }

    #[test]
    fn runtime_exports_ignores_cjs_syntax() {
        let tester = parse("export = 5;");
        tester.assert_empty();
        let tester = parse("import a = require('./a'); export as namespace A;");
        tester.assert_empty();
    }
}
