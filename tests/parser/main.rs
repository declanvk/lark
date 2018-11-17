use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use lark_entity::{Entity, EntityData};
use lark_parser::ParserDatabase;
use lark_span::FileName;
use lark_test::*;

#[derive(Debug, DebugWith, PartialEq, Eq)]
struct EntityTree {
    name: String,
    children: Vec<EntityTree>,
}

impl EntityTree {
    fn from_file(db: &impl ParserDatabase, file: FileName) -> Self {
        let entity = EntityData::InputFile { file: file.id }.intern(db);
        Self::from_entity(db, entity)
    }

    fn from_entity(db: &impl ParserDatabase, entity: Entity) -> Self {
        EntityTree {
            name: entity.untern(db).relative_name(db),
            children: db
                .child_entities(entity)
                .iter()
                .map(|&e| EntityTree::from_entity(db, e))
                .collect(),
        }
    }
}

#[test]
fn empty_struct() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn one_field() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
            x: uint
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: [
                            EntityTree {
                                name: "MemberName(x)",
                                children: []
                            }
                        ]
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn two_fields() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
            x: uint,
            y: uint
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: [
                            EntityTree {
                                name: "MemberName(x)",
                                children: []
                            },
                            EntityTree {
                                name: "MemberName(y)",
                                children: []
                            }
                        ]
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn one_struct_newline_variations() {
    let tree_base = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo {
                x: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo
            {

                x: uint


            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo
            {

                x
                :
                uint


            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);
}

#[test]
fn two_fields_variations() {
    let tree_base = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
                y: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint,
                y: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint,
                y: uint,
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
                y: uint,
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {



                x: uint


                y: uint,

            }


            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);
}

#[test]
fn two_structs_overlapping_lines() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
        } struct Bar {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: []
                    },
                    EntityTree {
                        name: "ItemName(Bar)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn two_structs_whitespace() {
    let base_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            } struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            }
            struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            }

            struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);
}

#[test]
fn eof_extra_sigil() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
            struct Foo {
                x: uint
            }

            +
            ",
    ));

    // These errors are (a) too numerous and (b) poor quality :(

    let entity = EntityData::InputFile { file: file_name.id }.intern(&db);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"
            [
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                }
            ]"#,
        ),
        &db.child_parsed_entities(entity).errors,
    );
}

#[test]
fn some_function() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        fn foo() {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(foo)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn function_variations() {
    let base_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() { }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo(x: uint) { }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo(
                x: uint,
            ) -> uint {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);
}
