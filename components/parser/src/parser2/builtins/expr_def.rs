use crate::prelude::*;

use crate::parser::ParseError;
use crate::parser2::allow::ALLOW_NEWLINE;
use crate::parser2::reader::{self, Reader, ShapeStart};
use crate::parser2::{Handle, LiteParser, ScopeId};

use derive_new::new;
use log::trace;

#[derive(Debug, new)]
pub struct ExprParser;

/// An expression has a well-defined lexical extent. This means that the
/// extent of a token list in expression position can be determined purely
/// by following these rules.
///
/// An expression has this structure:
///
/// - Ident
/// - PREFIX <operator-defined>
///
/// PREFIX means a registered prefix token, which includes at least:
///
/// - `!`
/// - `- [NoWhitespaceAllowed]`
/// - ownership tokens (TBD)
///
/// `operator-defined` means that what comes AFTER the prefix is in the
/// control of the operator, but it must be consistent across all uses of
/// the operator in prefix position.
///
/// An expression can be continued with:
///
/// - { ... }
/// - ( ... )
/// - [ ... ]
/// - OP <operator-defined>
///
/// OP means a registered operator token, which includes at least:
///
/// - `+` / `+=`
/// - `-` / `-=`
/// - `*` / `*=`
/// - `/` / `/=`
///
/// `operator-defined` means that what comes AFTER the operator is in the
/// control of the operator implementation, but it must be consistent across
/// all uses of the operator in prefix position.
impl ExprParser {
    pub fn extent(&mut self, reader: &mut Reader<'_>) -> Result<Handle, ParseError> {
        trace!(target: "lark::reader", "parsing expression");

        reader.tree().start("expression");
        reader.tree().mark_expr();

        self.process(reader)?;

        let handle = reader.tree().end("expression");

        Ok(handle)
    }

    fn process(&mut self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        self.start_expr(reader)?;

        Ok(())
    }

    fn start_expr(&mut self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        match reader.peek_start_expr(ALLOW_NEWLINE)? {
            ShapeStart::Macro(m) => Err(ParseError::new(
                format!("Unimplemented expression macro"),
                m.span(),
            )),
            id @ ShapeStart::Identifier(_) => self.start_id(reader, id),
            _ => Err(ParseError::new(
                format!("Unimplemented rest of start_expr"),
                reader.current_span(),
            )),
        }
    }

    fn start_id(&mut self, reader: &mut Reader<'_>, start: ShapeStart) -> Result<(), ParseError> {
        reader.consume_start_expr(start, ALLOW_NEWLINE)
    }
}

enum States {
    Initial,
    Delimited(Vec<Delimiter>),
    PossibleEnd,
}

enum Delimiter {
    Curly,
    Round,
    Square,
}
