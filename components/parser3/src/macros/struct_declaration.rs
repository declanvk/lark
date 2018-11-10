use crate::macros::type_reference::ParsedTypeReference;
use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::LazyParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::Spanned;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_string::global::GlobalIdentifier;
use std::sync::Arc;

/// ```ignore
/// struct <id> {
///   <id>: <ty> // separated by `,` or newline
/// }
/// ```
#[derive(Default)]
pub struct StructDeclaration;

impl EntityMacroDefinition for StructDeclaration {
    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier>,
    ) -> ParsedEntity {
        let struct_name = or_error_entity!(
            parser.eat_global_identifier(),
            parser,
            "expected struct name"
        );

        or_error_entity!(parser.eat_sigil("{"), parser, "expected `{`");
        parser.eat_newlines();

        let mut fields = vec![];
        loop {
            if let Some(name) = parser.eat_global_identifier() {
                if let Some(ty) = parser.parse_type() {
                    fields.push(ParsedField { name, ty });

                    // If there is a `,` or a newline, then there may
                    // be more fields, so go back around the loop.
                    if let Some(_) = parser.eat_sigil(",") {
                        parser.eat_newlines();
                        continue;
                    } else if parser.eat_newlines() {
                        continue;
                    }
                }
            }

            break;
        }

        if let None = parser.eat_sigil("}") {
            parser.report_error("expected `}`", parser.peek_span());
        }

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Struct,
            id: struct_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = struct_name.span;

        ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            Arc::new(ParsedStructDeclaration { fields }),
        )
    }
}

struct ParsedStructDeclaration {
    fields: Vec<ParsedField>,
}

struct ParsedField {
    name: Spanned<GlobalIdentifier>,
    ty: ParsedTypeReference,
}

impl LazyParsedEntity for ParsedStructDeclaration {
    fn parse_children(&self) -> Vec<ParsedEntity> {
        unimplemented!()
    }
}
