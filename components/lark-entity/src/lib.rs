#![feature(macro_at_most_once_rep)]
#![feature(specialization)]
#![feature(const_fn)]
#![feature(const_let)]

use debug::DebugWith;
use intern::Has;
use intern::Untern;
use lark_debug_derive::DebugWith;
use parser::StringId;

indices::index_type! {
    pub struct Entity { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum EntityData {
    InputFile { file: StringId },
    ItemName { base: Entity, id: StringId },
    MemberName { base: Entity, id: StringId },
}

intern::intern_tables! {
    pub struct EntityTables {
        struct EntityTablesData {
            item_ids: map(Entity, EntityData),
        }
    }
}

debug::debug_fallback_impl!(Entity);

impl<Cx> DebugWith<Cx> for Entity
where
    Cx: Has<EntityTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        data.fmt_with(cx, fmt)
    }
}

impl Entity {
    pub fn input_file(self, db: &dyn Has<EntityTables>) -> StringId {
        match self.untern(db) {
            EntityData::InputFile { file } => file,
            EntityData::ItemName { base, id: _ } => base.input_file(db),
            EntityData::MemberName { base, id: _ } => base.input_file(db),
        }
    }
}
