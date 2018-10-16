use crate::item_id::ItemId;
use crate::item_id::ItemIdData;
use crate::AstDatabase;
use intern::Intern;
use intern::Untern;
use parser::ast;
use parser::ParseError;
use parser::StringId;
use std::sync::Arc;

crate fn ast_of_file(
    db: &impl AstDatabase,
    path: StringId,
) -> Result<Arc<ast::Module>, ParseError> {
    let input_text = db.input_text(path).unwrap_or_else(|| {
        panic!("no input text for path `{}`", db.untern_string(path));
    });

    let module = db.parser_state().parse(path, input_text)?;

    Ok(Arc::new(module))
}

crate fn items_in_file(db: &impl AstDatabase, input_file: StringId) -> Arc<Vec<ItemId>> {
    let ast_of_file = match db.ast_of_file(input_file) {
        Ok(module) => module,
        Err(_) => return Arc::new(vec![]),
    };

    let input_file_id = ItemIdData::InputFile { file: input_file }.intern(db);

    let items: Vec<_> = ast_of_file
        .items
        .iter()
        .map(|item| {
            ItemIdData::ItemName {
                base: input_file_id,
                id: item.name(),
            }
            .intern(db)
        })
        .collect();
    Arc::new(items)
}

crate fn ast_of_item(db: &impl AstDatabase, item_id: ItemId) -> Result<Arc<ast::Item>, ParseError> {
    match item_id.untern(db) {
        ItemIdData::ItemName { base, id: path_id } => {
            match base.untern(db) {
                ItemIdData::InputFile { file: input_file } => {
                    // Base case: root item in a file

                    let module = db.ast_of_file(input_file)?;

                    for item in &module.items {
                        if item.name() == path_id {
                            return Ok(item.clone());
                        }
                    }

                    panic!("no such item")
                }

                _ => {
                    // Nested items -- don't implement for now, too lazy =)
                    unimplemented!()
                }
            }
        }

        d => panic!("ast-of-item invoked with non-item {:?}", d),
    }
}
