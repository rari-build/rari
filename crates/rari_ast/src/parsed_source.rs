// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::{
    fmt,
    sync::{Arc, OnceLock},
};

use oxc::{
    ast::ast::{Program, Statement},
    span::Span,
    syntax::module_record::ModuleRecord,
};

use crate::{MediaType, ModuleSpecifier, ParseDiagnostic, SourceTextInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    Esm,
    Cjs,
}

impl ModuleKind {
    pub fn from_is_cjs(is_cjs: bool) -> Self {
        if is_cjs { Self::Cjs } else { Self::Esm }
    }

    pub fn from_is_esm(is_esm: bool) -> Self {
        Self::from_is_cjs(!is_esm)
    }

    pub fn is_cjs(&self) -> bool {
        matches!(self, Self::Cjs)
    }

    pub fn is_esm(&self) -> bool {
        matches!(self, Self::Esm)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseDiagnostics {
    pub diagnostics: Vec<ParseDiagnostic>,
    /// Diagnostics that should be surfaced when transpiling if the
    /// file parsed as a script is discovered to be a module.
    pub script_module_diagnostics: Vec<ParseDiagnostic>,
}

impl ParseDiagnostics {
    #[cfg(feature = "transpiling")]
    pub fn for_module_kind<'a>(
        &'a self,
        module_kind: ModuleKind,
    ) -> Box<dyn Iterator<Item = &'a ParseDiagnostic> + 'a> {
        match module_kind {
            ModuleKind::Esm => {
                Box::new(self.diagnostics.iter().chain(self.script_module_diagnostics.iter()))
            }
            ModuleKind::Cjs => Box::new(self.diagnostics.iter()),
        }
    }
}

/// A parsed source containing an AST, comments, and possibly tokens.
pub struct ParsedSource<'a> {
    pub(crate) specifier: ModuleSpecifier,
    pub(crate) media_type: MediaType,
    pub(crate) text: Arc<str>,
    pub(crate) source_text_info: OnceLock<SourceTextInfo>,
    pub(crate) program: Program<'a>,
    pub(crate) module_record: ModuleRecord<'a>,
    pub(crate) tokens: oxc::allocator::Vec<'a, oxc::parser::Token>,
    pub(crate) diagnostics: ParseDiagnostics,
    pub(crate) scoping: Option<oxc::semantic::Scoping>,
}

impl<'a> ParsedSource<'a> {
    /// Gets the module specifier of the module.
    pub fn specifier(&self) -> &ModuleSpecifier {
        &self.specifier
    }

    /// Gets the media type of the module.
    pub fn media_type(&self) -> MediaType {
        self.media_type
    }

    /// Gets the text content of the module.
    pub fn text(&self) -> &Arc<str> {
        &self.text
    }

    /// Note: Prefer using `.text()` over this if able because this is lazily
    /// created.
    pub fn text_info_lazy(&self) -> &SourceTextInfo {
        self.source_text_info.get_or_init(|| SourceTextInfo::new(Arc::clone(&self.text)))
    }

    /// Gets the parsed program.
    pub fn program(&self) -> &Program<'a> {
        &self.program
    }

    pub fn module_record(&self) -> &ModuleRecord<'a> {
        &self.module_record
    }

    pub fn scoping(&self) -> Option<&oxc::semantic::Scoping> {
        self.scoping.as_ref()
    }

    pub fn take_scoping(&mut self) -> Option<oxc::semantic::Scoping> {
        self.scoping.take()
    }

    pub fn program_mut(&mut self) -> &mut Program<'a> {
        &mut self.program
    }

    pub fn tokens(&self) -> &oxc::allocator::Vec<'a, oxc::parser::Token> {
        &self.tokens
    }

    /// Gets the comments found in the source file.
    pub fn comments(&self) -> &oxc::allocator::Vec<'a, oxc::ast::ast::Comment> {
        &self.program.comments
    }

    pub fn body(&self) -> &oxc::allocator::Vec<'a, Statement<'a>> {
        &self.program.body
    }

    pub fn span(&self) -> Span {
        self.program.span
    }

    pub fn is_module(&self) -> bool {
        self.program.source_type.is_module()
    }

    pub fn is_script(&self) -> bool {
        self.program.source_type.is_script()
    }

    /// Computes whether this program should be treated as a script.
    pub fn compute_is_script(&self) -> bool {
        if self.media_type.is_typed() {
            compute_is_script_from_body(&self.program.body)
        } else {
            self.is_script()
        }
    }

    /// Gets extra non-fatal diagnostics found while parsing.
    pub fn diagnostics(&self) -> &Vec<ParseDiagnostic> {
        &self.diagnostics.diagnostics
    }

    /// Diagnostics that should be surfaced when transpiling if the
    /// file parsed as a script is discovered to be a module.
    pub fn script_module_diagnostics(&self) -> &Vec<ParseDiagnostic> {
        &self.diagnostics.script_module_diagnostics
    }

    /// Get the source's leading comments, where triple slash directives might
    /// be located.
    pub fn get_leading_comments(&self) -> impl Iterator<Item = &oxc::ast::ast::Comment> {
        let first_stmt_start = self
            .program
            .body
            .first()
            .map(|s| {
                use oxc::span::GetSpan;
                s.span().start
            })
            .unwrap_or(self.program.span.end);

        self.program.comments.iter().filter(move |c| c.span.start < first_stmt_start)
    }
}

pub(crate) fn compute_is_script_from_body(body: &[Statement<'_>]) -> bool {
    use oxc::ast::ast::*;
    !body.iter().any(|stmt| match stmt {
        Statement::ImportDeclaration(_)
        | Statement::ExportNamedDeclaration(_)
        | Statement::ExportFromDeclaration(_)
        | Statement::ExportDefaultDeclaration(_)
        | Statement::ExportAllDeclaration(_) => true,
        Statement::ExportDeclaration(decl) => {
            !matches!(decl.declaration, Declaration::TSImportEqualsDeclaration(_))
        }
        _ => false,
    })
}

impl fmt::Debug for ParsedSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ParsedSource")
            .field("specifier", &self.specifier)
            .field("media_type", &self.media_type)
            .finish()
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod test {
    use oxc::allocator::Allocator;

    use super::*;
    use crate::{ParseParams, parse_program};

    #[test]
    fn compute_is_script() {
        fn get(text: &str, ext: &str) -> bool {
            let specifier = ModuleSpecifier::parse(&format!("file:///my_file.{ext}")).unwrap();
            let media_type = MediaType::from_specifier(&specifier);
            let allocator = Allocator::default();
            let program = parse_program(
                &allocator,
                ParseParams {
                    specifier,
                    text: text.into(),
                    media_type,
                    capture_tokens: true,
                    maybe_source_type: None,
                    scope_analysis: false,
                },
            )
            .unwrap();
            program.compute_is_script()
        }

        // false, tla
        assert!(!get("const mod = await import('./soljson.js');\nconsole.log(mod)", "js"));

        // false, import
        assert!(!get("import './test';", "js"));
        assert!(!get("import './test';", "ts"));

        // true, require
        assert!(get("require('test')", "js"));
        assert!(get("require('test')", "ts"));

        // true, ts import equals
        assert!(get("import value = require('test');", "ts"));
    }
}
