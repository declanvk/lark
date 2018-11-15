use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::ErrorParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::entity::ParsedEntityThunk;
use crate::syntax::field::Field;
use crate::syntax::field::ParsedField;
use crate::syntax::fn_body;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::matched::Matched;
use crate::syntax::matched::ParsedMatch;
use crate::syntax::sigil::Curlies;
use crate::syntax::sigil::Parentheses;
use crate::syntax::sigil::RightArrow;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::FileName;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_error::ErrorReported;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_hir as hir;
use lark_seq::Seq;
use lark_span::Spanned;
use lark_string::global::GlobalIdentifier;

/// ```ignore
/// `def` <id> `(` <id> `:` <ty> `)` [ `->` <ty> ] <block>
/// ```
#[derive(Default)]
pub struct FunctionDeclaration;

impl EntityMacroDefinition for FunctionDeclaration {
    fn expect(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier>,
    ) -> Result<ParsedEntity, ErrorReported> {
        log::trace!(
            "FunctionDeclaration::parse(base={}, macro_name={})",
            base.debug_with(parser),
            macro_name.debug_with(parser)
        );

        let function_name = parser.expect(SkipNewline(SpannedGlobalIdentifier))?;

        let parameters = parser
            .expect(SkipNewline(Delimited(Parentheses, CommaList(Field))))
            .unwrap_or_else(|ErrorReported(_)| Seq::default());

        let return_type = match parser
            .parse_if_present(SkipNewline(Guard(RightArrow, SkipNewline(TypeReference))))
        {
            Some(ty) => ty.unwrap_or_error_sentinel(&*parser),
            None => ParsedTypeReference::Elided(parser.elided_span()),
        };

        let body = parser.expect(SkipNewline(Matched(Curlies)));

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Function,
            id: function_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = function_name.span;

        Ok(ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            ParsedEntityThunk::new(ParsedFunctionDeclaration {
                parameters,
                return_type,
                body,
            }),
        ))
    }
}

struct ParsedFunctionDeclaration {
    parameters: Seq<Spanned<ParsedField>>,
    return_type: ParsedTypeReference,
    body: Result<Spanned<ParsedMatch>, ErrorReported>,
}

impl LazyParsedEntity for ParsedFunctionDeclaration {
    fn parse_children(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        WithError::ok(vec![])
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        match self.body {
            Err(_) => ErrorParsedEntity.parse_fn_body(entity, db),

            Ok(Spanned {
                span: _,
                value:
                    ParsedMatch {
                        start_token,
                        end_token,
                    },
            }) => {
                let file_name = FileName {
                    id: entity.untern(db).file_name(db).unwrap(),
                };
                let input = db.file_text(file_name);
                let tokens = db
                    .file_tokens(file_name)
                    .into_value()
                    .extract(start_token..end_token);
                let entity_macro_definitions = crate::macro_definitions(db, entity);
                let arguments: Seq<_> = self.parameters.iter().map(|f| f.value.name).collect();
                fn_body::parse_fn_body(
                    entity,
                    db,
                    &entity_macro_definitions,
                    &input,
                    &tokens,
                    arguments,
                )
            }
        }
    }
}
