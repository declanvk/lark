use lark_debug_derive::DebugWith;
use lark_error::Diagnostic;
use lark_error::ErrorSentinel;

/// The different kinds of token our lexer can distinguish. Note that
/// you can recover the full text of the token using its span.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq)]
pub enum LexToken {
    Whitespace,
    Identifier,
    Sigil,
    Comment,
    String,
    Newline,
    EOF,
    Error,
}

impl<Cx> ErrorSentinel<Cx> for LexToken {
    fn error_sentinel(_cx: Cx, _error_spans: &[Diagnostic]) -> Self {
        LexToken::Error
    }
}
