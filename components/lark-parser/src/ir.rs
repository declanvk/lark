use crate::syntax::entity::ParsedEntity;

use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_seq::Seq;
use lark_span::{FileName, Span};

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, new)]
pub struct ParsedFile {
    file_name: FileName,
    entities: Seq<ParsedEntity>,
    span: Span<FileName>,
}

impl ParsedFile {
    pub fn entities(&self) -> &Seq<ParsedEntity> {
        &self.entities
    }
}
