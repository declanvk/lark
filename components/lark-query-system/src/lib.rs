use ast::{HasParserState, InputText, ParserState};
use codespan::{CodeMap, FileMap, FileName};
use lark_entity::EntityTables;
use lark_task_manager::{Actor, NoopSendChannel, QueryRequest, QueryResponse, SendChannel};
use map::FxIndexMap;
use parking_lot::RwLock;
use parser::pos::Span;
use salsa::{Database, ParallelDatabase};
use std::borrow::Cow;
use std::sync::Arc;
use ty::interners::TyInternTables;

mod ls_ops;
use self::ls_ops::{Cancelled, LsDatabase};

#[derive(Default)]
struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    code_map: Arc<RwLock<CodeMap>>,
    file_maps: Arc<RwLock<FxIndexMap<String, Arc<FileMap>>>>,
    parser_state: Arc<ParserState>,
    item_id_tables: Arc<EntityTables>,
    ty_intern_tables: Arc<TyInternTables>,
}

impl Database for LarkDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<LarkDatabase> {
        &self.runtime
    }
}

impl ParallelDatabase for LarkDatabase {
    fn fork(&self) -> Self {
        LarkDatabase {
            code_map: self.code_map.clone(),
            file_maps: self.file_maps.clone(),
            runtime: self.runtime.fork(),
            parser_state: self.parser_state.clone(),
            item_id_tables: self.item_id_tables.clone(),
            ty_intern_tables: self.ty_intern_tables.clone(),
        }
    }
}

impl LsDatabase for LarkDatabase {
    fn file_maps(&self) -> &RwLock<FxIndexMap<String, Arc<FileMap>>> {
        &self.file_maps
    }
}

salsa::database_storage! {
    struct LarkDatabaseStorage for LarkDatabase {
        impl ast::AstDatabase {
            fn input_files() for ast::InputFilesQuery;
            fn input_text() for ast::InputTextQuery;
            fn ast_of_file() for ast::AstOfFileQuery;
            fn items_in_file() for ast::ItemsInFileQuery;
            fn ast_of_item() for ast::AstOfItemQuery;
            fn ast_of_field() for ast::AstOfFieldQuery;
            fn entity_span() for ast::EntitySpanQuery;
        }
        impl hir::HirDatabase {
            fn boolean_entity() for hir::BooleanEntityQuery;
            fn fn_body() for hir::FnBodyQuery;
            fn members() for hir::MembersQuery;
            fn member_entity() for hir::MemberEntityQuery;
            fn subentities() for hir::SubentitiesQuery;
            fn ty() for hir::TyQuery;
            fn signature() for hir::SignatureQuery;
            fn generic_declarations() for hir::GenericDeclarationsQuery;
            fn resolve_name() for hir::ResolveNameQuery;
        }
        impl type_check::TypeCheckDatabase {
            fn base_type_check() for type_check::BaseTypeCheckQuery;
        }
    }
}

impl parser::LookupStringId for LarkDatabase {
    fn lookup(&self, id: parser::StringId) -> Arc<String> {
        self.untern_string(id)
    }
}

impl AsRef<EntityTables> for LarkDatabase {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

impl AsRef<TyInternTables> for LarkDatabase {
    fn as_ref(&self) -> &TyInternTables {
        &self.ty_intern_tables
    }
}

impl HasParserState for LarkDatabase {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

pub struct QuerySystem {
    send_channel: Box<dyn SendChannel<QueryResponse>>,
    lark_db: LarkDatabase,
}

impl QuerySystem {
    pub fn new() -> QuerySystem {
        QuerySystem {
            send_channel: Box::new(NoopSendChannel),
            lark_db: LarkDatabase::default(),
        }
    }
}

impl Actor for QuerySystem {
    type InMessage = QueryRequest;
    type OutMessage = QueryResponse;

    fn startup(&mut self, send_channel: &dyn SendChannel<QueryResponse>) {
        self.send_channel = send_channel.clone_send_channel();
    }

    fn shutdown(&mut self) {}

    fn receive_message(&mut self, message: Self::InMessage) {
        match message {
            QueryRequest::OpenFile(url, contents) => {
                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
                let interned_path = self.lark_db.intern_string(url.as_str());
                let interned_contents = self.lark_db.intern_string(contents.as_str());
                self.lark_db
                    .query(ast::InputFilesQuery)
                    .set((), Arc::new(vec![interned_path]));

                // Uh, adding a "new" file on each change seems a bit ungreat. But good
                // enough for now.
                let file_map = self.lark_db.code_map.write().add_filemap(
                    FileName::Virtual(Cow::Owned(url.to_string())),
                    contents.to_string(),
                );
                let file_span = file_map.span();
                let start_offset = file_map.span().start().to_usize() as u32;

                // Record the filemap for later
                self.lark_db
                    .file_maps
                    .write()
                    .insert(url.to_string(), file_map);

                self.lark_db.query(ast::InputTextQuery).set(
                    interned_path,
                    Some(InputText {
                        text: interned_contents,
                        start_offset,
                        span: Span::from(file_span),
                    }),
                );
            }
            QueryRequest::EditFile(_) => {}
            QueryRequest::TypeAtPosition(task_id, url, position) => {
                std::thread::spawn({
                    let db = self.lark_db.fork();
                    let send_channel = self.send_channel.clone_send_channel();
                    move || {
                        // Ensure that `type_at_position` executes atomically
                        let _lock = db.salsa_runtime().lock_revision();

                        match db.hover_text_at_position(url.as_str(), position) {
                            Ok(Some(v)) => {
                                send_channel.send(QueryResponse::Type(task_id, v.to_string()));
                            }
                            Ok(None) => {
                                // FIXME what to send here to indicate "no hover"?
                                send_channel.send(QueryResponse::Type(task_id, "".to_string()));
                            }
                            Err(Cancelled) => {
                                // Not sure what to send here, if anything.
                                send_channel
                                    .send(QueryResponse::Type(task_id, format!("<cancelled>")));
                            }
                        }
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
