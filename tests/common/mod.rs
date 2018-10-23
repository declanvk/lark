use lark_mir::{
    builtin_type, BasicBlock, BinOp, Context, DefId, Definition, Function, LocalDecl, Operand,
    Place, Rvalue, StatementKind, Struct, TerminatorKind,
};

pub fn generate_simple_add_test() -> (Context, DefId) {
    let mut context = Context::new();

    let void_ty = context.simple_type_for_def_id(builtin_type::VOID);
    let i32_ty = context.simple_type_for_def_id(builtin_type::I32);

    let mut m = Function::new(void_ty, vec![], "main".into());
    let add_result_tmp = m.new_temp(i32_ty);
    let lhs_tmp = m.new_temp(i32_ty);
    let rhs_tmp = m.new_temp(i32_ty);

    let mut bb1 = BasicBlock::new();
    bb1.push_stmt(StatementKind::Assign(
        Place::Local(lhs_tmp),
        Rvalue::Use(Operand::ConstantInt(11)),
    ));
    bb1.push_stmt(StatementKind::Assign(
        Place::Local(rhs_tmp),
        Rvalue::Use(Operand::ConstantInt(7)),
    ));

    bb1.push_stmt(StatementKind::Assign(
        Place::Local(add_result_tmp),
        Rvalue::BinaryOp(BinOp::Add, lhs_tmp, rhs_tmp),
    ));
    bb1.push_stmt(StatementKind::DebugPrint(Place::Local(add_result_tmp)));
    bb1.terminate(TerminatorKind::Return);
    m.push_block(bb1);

    let main_def_id = context.add_definition(Definition::Fn(m));

    (context, main_def_id)
}

pub fn generate_big_test() -> (Context, DefId) {
    let mut context = Context::new();

    let i32_ty = context.simple_type_for_def_id(builtin_type::I32);
    let void_ty = context.simple_type_for_def_id(builtin_type::VOID);

    let mut bob = Function::new(
        i32_ty,
        vec![
            LocalDecl::new(i32_ty, Some("x".into())),
            LocalDecl::new(i32_ty, Some("y".into())),
        ],
        "bob".into(),
    );

    let bob_tmp = bob.new_temp(i32_ty);

    let mut bb1 = BasicBlock::new();

    bb1.push_stmt(StatementKind::Assign(
        Place::Local(bob_tmp),
        Rvalue::BinaryOp(BinOp::Sub, 1, 2),
    ));
    bb1.push_stmt(StatementKind::Assign(
        Place::Local(0),
        Rvalue::Use(Operand::Move(Place::Local(bob_tmp))),
    ));

    bb1.terminate(TerminatorKind::Return);

    bob.push_block(bb1);

    let bob_def_id = context.add_definition(Definition::Fn(bob));

    let person = Struct::new("Person".into())
        .field("height".into(), i32_ty)
        .field("id".into(), i32_ty);

    let person_def_id = context.add_definition(Definition::Struct(person));
    let person_ty = context.simple_type_for_def_id(person_def_id);

    let mut m = Function::new(void_ty, vec![], "main".into());
    let call_result_tmp = m.new_temp(i32_ty);
    let person_result_tmp = m.new_temp(person_ty);

    let mut bb2 = BasicBlock::new();

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(call_result_tmp),
        Rvalue::Call(
            bob_def_id,
            vec![Operand::ConstantInt(11), Operand::ConstantInt(8)],
        ),
    ));

    bb2.push_stmt(StatementKind::DebugPrint(Place::Local(call_result_tmp)));

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(person_result_tmp),
        Rvalue::Call(
            person_def_id,
            vec![Operand::ConstantInt(17), Operand::ConstantInt(18)],
        ),
    ));

    bb2.push_stmt(StatementKind::DebugPrint(Place::Field(
        person_result_tmp,
        "id".into(),
    )));

    bb2.terminate(TerminatorKind::Return);
    m.push_block(bb2);
    let main_def_id = context.add_definition(Definition::Fn(m));

    (context, main_def_id)
}
