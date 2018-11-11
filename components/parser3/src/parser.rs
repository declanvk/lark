use crate::lexer::definition::LexerState;
use crate::lexer::token::LexToken;
use crate::lexer::tools::Tokenizer;
use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::ErrorParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_error::Diagnostic;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;
use map::FxIndexMap;
use std::sync::Arc;

pub struct Parser<'me> {
    global_identifier_tables: &'me GlobalIdentifierTables,
    entity_tables: &'me EntityTables,
    entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
    input: &'me Text,
    tokenizer: Tokenizer<'me, LexerState>,
    errors: Vec<Diagnostic>,

    /// Current lookahead token.
    token: Spanned<LexToken>,

    /// The span of the last token that we consumed (i.e., the one
    /// immediately before `self.token`).
    last_span: Span<CurrentFile>,
}

impl Parser<'me> {
    crate fn new(
        db: &'me (impl AsRef<GlobalIdentifierTables> + AsRef<EntityTables>),
        entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
        input: &'me Text,
    ) -> Self {
        let mut tokenizer = Tokenizer::new(input);
        let mut errors = vec![];
        let token = next_token(&mut tokenizer, &mut errors, input);
        Parser {
            global_identifier_tables: db.as_ref(),
            entity_tables: db.as_ref(),
            entity_macro_definitions,
            input,
            tokenizer,
            errors,
            last_span: Span::initial(CurrentFile),
            token,
        }
    }

    /// Parse all the entities we can and return a vector
    /// (accumulating errors as we go).
    crate fn parse_all_entities(mut self, parent_entity: Entity) -> WithError<Vec<ParsedEntity>> {
        let mut entities = vec![];
        while let Some(entity) = self.parse_entity(parent_entity) {
            entities.push(entity);
        }

        WithError {
            value: entities,
            errors: self.errors,
        }
    }

    /// Consume the current token and load the next one.  Return the
    /// old token.
    crate fn shift(&mut self) -> Spanned<LexToken> {
        self.last_span = self.token.span;
        std::mem::replace(
            &mut self.token,
            next_token(&mut self.tokenizer, &mut self.errors, self.input),
        )
    }

    /// Extract the complete input
    crate fn input(&self) -> &'me Text {
        self.input
    }

    /// Peek at the current lookahead token.
    crate fn peek(&self) -> Spanned<LexToken> {
        self.token
    }

    /// Span of the current lookahead token.
    crate fn peek_span(&self) -> Span<CurrentFile> {
        self.token.span
    }

    /// Span of the last consumed token.
    crate fn last_span(&self) -> Span<CurrentFile> {
        self.token.span
    }

    /// Peek at the string reprsentation of the current token.
    crate fn peek_str(&self) -> &'me str {
        &self.input[self.token.span]
    }

    /// Test if the current token is of the given kind.
    crate fn is(&self, kind: LexToken) -> bool {
        kind == self.token.value
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
        if let Some(v) = self.eat(&syntax) {
            return Ok(v);
        }

        Err(self.report_error(
            format!("expected {}", syntax.singular_name()),
            self.token.span,
        ))
    }

    /// Parse a piece of syntax (if it is present)
    crate fn eat<T>(&mut self, syntax: T) -> Option<T::Data>
    where
        T: Syntax,
    {
        syntax.parse(self)
    }

    /// Parses an entity, if one is present, and returns its parsed
    /// representation. Otherwise, returns `None`.
    ///
    /// Entities always begin with a macro invocation and then proceed
    /// as the macro demands.
    crate fn parse_entity(&mut self, parent_entity: Entity) -> Option<ParsedEntity> {
        let macro_name = self.eat(SpannedGlobalIdentifier)?;

        let result = try {
            let macro_definition = match self.entity_macro_definitions.get(&macro_name.value) {
                Some(m) => m.clone(),
                None => Err(self.report_error("no macro with this name", macro_name.span))?,
            };

            macro_definition.parse(self, parent_entity, macro_name)?
        };

        match result {
            Ok(v) => Some(v),

            Err(report) => {
                let entity = EntityData::Error(report).intern(self);
                let full_span = macro_name.span.extended_until_end_of(self.last_span());
                Some(ParsedEntity::new(
                    entity,
                    full_span,
                    full_span,
                    Arc::new(ErrorParsedEntity),
                ))
            }
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

fn next_token(
    tokenizer: &mut Tokenizer<'_, LexerState>,
    errors: &mut Vec<Diagnostic>,
    input: &'me Text,
) -> Spanned<LexToken> {
    loop {
        let new_token = tokenizer.next().unwrap_or_else(|| {
            Ok(Spanned {
                value: LexToken::EOF,
                span: Span::eof(CurrentFile, input),
            })
        });

        // Skip over whitespace/comments automatically (but not
        // newlines).
        match new_token {
            Ok(token) => match token.value {
                LexToken::Whitespace | LexToken::Comment => {
                    continue;
                }

                _ => {
                    return token;
                }
            },

            Err(span) => {
                report_error(errors, "unrecognized token", span);
                continue;
            }
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
