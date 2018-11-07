use ast::AstDatabase;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::ls_ops::RangedDiagnostic;
use lark_query_system::LarkDatabase;
use parser::HasParserState;
use parser::HasReaderState;

trait ErrorSpec {
    fn check_errors(&self, errors: &[RangedDiagnostic]);
}

struct NoErrors;

impl ErrorSpec for NoErrors {
    fn check_errors(&self, errors: &[RangedDiagnostic]) {
        if errors.is_empty() {
            return;
        }

        for error in errors {
            eprintln!("{:?}", error);
        }

        assert_eq!(0, errors.len());
    }
}

impl ErrorSpec for &str {
    fn check_errors(&self, errors: &[RangedDiagnostic]) {
        assert_eq!(
            errors.len(),
            1,
            "expected exactly one error, got {:#?}",
            errors
        );

        for error in errors {
            let range = error.range;

            let expected = format!("0:{}..0:{}", self.find('~').unwrap(), self.len());
            let actual = format!(
                "{}:{}..{}:{}",
                range.start.line, range.start.character, range.end.line, range.end.character
            );

            if expected != actual {
                eprintln!("expected error on {}", expected);
                eprintln!("found error on {}", actual);
                eprintln!("error = {:#?}", error);
            }

            assert_eq!(expected, actual);
        }
    }
}

fn run_test(text: &str, error_spec: impl ErrorSpec) {
    let mut db = LarkDatabase::default();
    let path1_str = "path1";
    let path1_interned = db.intern_string("path1");

    db.add_file(path1_str, text);

    let items_in_file = db.items_in_file(path1_interned);
    assert!(items_in_file.len() >= 1, "input with no items");

    match db.errors_for_project() {
        Ok(errors) => {
            let flat_errors: Vec<_> = errors
                .into_iter()
                .flat_map(|(file_name, errors)| {
                    assert_eq!(file_name, path1_str);
                    errors
                })
                .collect();
            error_spec.check_errors(&flat_errors);
        }

        Err(Cancelled) => {
            panic!("cancelled?!");
        }
    }
}

#[test]
fn bad_identifier() {
    run_test(
        "def new(msg: bool,) -> bool { msg1 }",
        "                              ~~~~",
    );
}

#[test]
fn bad_callee() {
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) }",
        "                              ~~~",
    );
}

#[test]
fn correct_call() {
    run_test(
        "def foo(msg: bool,) { bar(msg) } def bar(arg:bool,) { }",
        NoErrors,
    );
}

#[test]
fn wrong_num_of_arguments() {
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) } def bar(arg:bool, arg2:bool) { }",
        "                              ~~~~~~~~",
    );
}

#[test]
fn wrong_return_type() {
    // `bar` returns unit, we expect `bool`
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) } def bar(arg:bool,) { }",
        "                              ~~~~~~~~",
    );
}

#[test]
fn wrong_type_of_arguments() {
    run_test(
        "def foo(msg: int,) { bar(msg) } def bar(arg:bool,) { }",
        "                         ~~~",
    );
}
