use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use crate::syntax::entity::EntitySyntax;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::Syntax;
use debug::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_error::Diagnostic;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_seq::Seq;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;
use map::FxIndexMap;
use std::sync::Arc;

pub struct Parser<'me> {
    /// Tables for interning global identifiers; extracted from the database.
    global_identifier_tables: &'me GlobalIdentifierTables,

    /// Tables for interning entities; extracted from the database.
    entity_tables: &'me EntityTables,

    /// Set of macro definitions in scope.
    entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,

    /// Complete input; needed to extract the full text of tokens.
    input: &'me Text,

    /// List of all tokens.
    tokens: &'me Seq<Spanned<LexToken>>,

    /// Index of the next token to consume.
    next_lookahead_token: usize,

    /// Span of the last consumed token (ignoring whitespace and
    /// comments); see the `last_span()` method below.
    last_span: Span<CurrentFile>,

    /// Current lookahead token.
    lookahead_token: Spanned<LexToken>,

    /// Errors reported during parsing; these will be converted into
    /// the final `WithError` result
    errors: Vec<Diagnostic>,
}

impl Parser<'me> {
    crate fn new(
        db: &'me (impl AsRef<GlobalIdentifierTables> + AsRef<EntityTables>),
        entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
        input: &'me Text,
        tokens: &'me Seq<Spanned<LexToken>>,
        start_token: usize,
    ) -> Self {
        // Subtle: the start token may be whitespace etc. So we actually have to invoke
        // `advance_next_token` to advance.
        let mut next_lookahead_token = start_token;
        let lookahead_token = advance_next_token(tokens, &mut next_lookahead_token);

        Parser {
            global_identifier_tables: db.as_ref(),
            entity_tables: db.as_ref(),
            entity_macro_definitions,
            input,
            tokens,
            next_lookahead_token,
            lookahead_token,
            errors: vec![],
            last_span: Span::initial(CurrentFile),
        }
    }

    /// Parse all the entities we can and return a vector
    /// (accumulating errors as we go).
    crate fn parse_all_entities(mut self, parent_entity: Entity) -> WithError<Seq<ParsedEntity>> {
        let mut entities = vec![];
        while let Some(entity) = self.eat(EntitySyntax::new(parent_entity)) {
            match entity {
                Ok(entity) => entities.push(entity),
                Err(ErrorReported(_)) => {}
            }
        }

        WithError {
            value: Seq::from(entities),
            errors: self.errors,
        }
    }

    /// Consume the current token and load the next one.  Return the
    /// old token.
    crate fn shift(&mut self) -> Spanned<LexToken> {
        self.last_span = self.lookahead_token.span;
        let last_token = self.lookahead_token;

        self.lookahead_token = advance_next_token(self.tokens, &mut self.next_lookahead_token);

        log::trace!(
            "shift: new lookahead token = {}, consumed token = {}",
            self.lookahead_token.debug_with(self),
            last_token.debug_with(self),
        );

        last_token
    }

    /// Extract the complete input
    crate fn input(&self) -> &'me Text {
        self.input
    }

    /// Extract the complete input
    crate fn entity_macro_definitions(
        &self,
    ) -> &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>> {
        self.entity_macro_definitions
    }

    /// Peek at the current lookahead token.
    crate fn peek(&self) -> Spanned<LexToken> {
        self.lookahead_token
    }

    /// Span of the current lookahead token.
    crate fn peek_span(&self) -> Span<CurrentFile> {
        self.peek().span
    }

    /// Span of the last consumed token, ignoring whitespace and
    /// comments. This is very handy when constructing the span of
    /// things we are looking at.  You basically consume tokens until
    /// the lookahead tells you that you are at the end, and then you
    /// can look at the `last_span`
    crate fn last_span(&self) -> Span<CurrentFile> {
        self.last_span
    }

    /// Peek at the string reprsentation of the current token.
    crate fn peek_str(&self) -> &'me str {
        &self.input[self.peek_span()]
    }

    /// Test if the current token is of the given kind.
    crate fn is(&self, kind: LexToken) -> bool {
        kind == self.lookahead_token.value
    }

    crate fn test(&self, syntax: impl Syntax) -> bool {
        log::trace!("test({})", syntax.debug_with(self));

        if syntax.test(self) {
            log::trace!("test: passed");
            true
        } else {
            false
        }
    }

    /// Consumes all subsequent newline characters, returning true if
    /// at least one newline was found.
    crate fn eat_newlines(&mut self) -> bool {
        let mut count = 0;
        while self.is(LexToken::Newline) {
            self.shift();
            count += 1;
        }
        count > 0
    }

    /// Parses a `T` if we can and returns true if so; otherwise,
    /// reports an error and returns false.
    crate fn expect<T>(&'s mut self, syntax: T) -> Result<T::Data, ErrorReported>
    where
        T: Syntax,
    {
        log::trace!("expect({})", syntax.debug_with(self));

        syntax.parse(self)
    }

    /// Parse a piece of syntax (if it is present)
    crate fn eat<T>(&mut self, syntax: T) -> Option<Result<T::Data, ErrorReported>>
    where
        T: Syntax,
    {
        log::trace!("eat({})", syntax.debug_with(self));

        if self.test(&syntax) {
            Some(self.expect(syntax))
        } else {
            None
        }
    }

    /// Report an error with the given message at the given span.
    crate fn report_error(
        &mut self,
        message: impl Into<String>,
        span: Span<CurrentFile>,
    ) -> ErrorReported {
        report_error(&mut self.errors, message, span)
    }
}

impl AsRef<GlobalIdentifierTables> for Parser<'_> {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.global_identifier_tables
    }
}

impl AsRef<EntityTables> for Parser<'_> {
    fn as_ref(&self) -> &EntityTables {
        self.entity_tables
    }
}

fn advance_next_token(tokens: &[Spanned<LexToken>], next_token: &mut usize) -> Spanned<LexToken> {
    loop {
        let token = tokens[*next_token];

        // Advance to the next token, unless we are at EOF.
        *next_token = (*next_token + 1).min(tokens.len() - 1);

        // Skip over whitespace/comments automatically (but not
        // newlines).
        match token.value {
            LexToken::Whitespace | LexToken::Comment => continue,
            _ => return token,
        }
    }
}

fn report_error(
    errors: &mut Vec<Diagnostic>,
    message: impl Into<String>,
    span: Span<CurrentFile>,
) -> ErrorReported {
    let message: String = message.into();
    let diagnostic = crate::diagnostic(message, span);
    errors.push(diagnostic);
    ErrorReported::at_diagnostic(errors.last().unwrap())
}
