use std::borrow::Cow;

use oxc::str::JSStr;

pub trait JSStrExt<'a> {
    fn to_str_lossy(self) -> Cow<'a, str>;

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
        assert_eq!(string_literal_value(r#"0; "a\uD83Db";"#), "a\u{FFFD}b");
        assert_eq!(string_literal_value(r#"0; "\uD83D\uDE00";"#), "😀");
    }
}
