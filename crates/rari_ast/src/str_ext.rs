// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;

use oxc::str::JSStr;

/// Extension methods for oxc's [`JSStr`], which is the value type of string
/// literals in the AST. A `JSStr` may contain lone surrogates and therefore
/// cannot always be borrowed as a `&str` directly.
pub trait JSStrExt<'a> {
    /// Borrows the string as UTF-8 when it contains no lone surrogates,
    /// otherwise allocates a copy with each lone surrogate replaced by U+FFFD.
    fn to_str_lossy(self) -> Cow<'a, str>;

    /// Converts the string to an owned `String`, replacing each lone surrogate
    /// with U+FFFD.
    fn to_string_lossy(self) -> String;
}

impl<'a> JSStrExt<'a> for JSStr<'a> {
    fn to_str_lossy(self) -> Cow<'a, str> {
        match self.as_str() {
            Some(text) => Cow::Borrowed(text),
            None => Cow::Owned(
                self.chars().map(|c| c.to_char().unwrap_or(char::REPLACEMENT_CHARACTER)).collect(),
            ),
        }
    }

    fn to_string_lossy(self) -> String {
        self.to_str_lossy().into_owned()
    }
}

#[cfg(test)]
mod test {
    use oxc::{
        allocator::Allocator,
        ast::ast::{Expression, Statement},
        parser::Parser,
        span::SourceType,
    };

    use super::*;

    fn string_literal_value(source: &str) -> String {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::mjs()).parse();
        let Statement::ExpressionStatement(stmt) = &ret.program.body[1] else { unreachable!() };
        let Expression::StringLiteral(lit) = &stmt.expression else { unreachable!() };
        lit.value.to_string_lossy()
    }

    #[test]
    fn to_str_lossy_borrows_valid_utf8() {
        let value = JSStr::from("hello");
        assert!(matches!(value.to_str_lossy(), Cow::Borrowed("hello")));
    }

    #[test]
    fn to_string_lossy_replaces_lone_surrogates() {
        // the leading statement keeps the string literal from being a directive
        assert_eq!(string_literal_value(r#"0; "a\uD83Db";"#), "a\u{FFFD}b");
        assert_eq!(string_literal_value(r#"0; "\uD83D\uDE00";"#), "😀");
    }
}
