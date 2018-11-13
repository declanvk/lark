use crate::parser::Parser;
use crate::syntax::Syntax;
use debug::DebugWith;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

/// Skips over any newlines
#[derive(DebugWith)]
pub struct SkipNewline<T>(pub T);

impl<T> SkipNewline<T> {
    fn content(&self) -> &T {
        &self.0
    }
}

impl<T> Syntax for SkipNewline<T>
where
    T: Syntax + DebugWith,
{
    type Data = T::Data;

    fn test(&self, parser: &Parser<'_>) -> bool {
        let mut parser = parser.checkpoint();
        parser.skip_newlines();
        parser.test(self.content())
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        parser.skip_newlines();
        parser.expect(self.content())
    }
}
