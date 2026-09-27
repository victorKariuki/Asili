//! Bitwise precedence and compound assignment (including on index targets).

use asili_lexer::tokenize;
use asili_parser::{parse_tokens, AssignOp, BinaryOp, Expr, Stmt};

fn body(src: &str) -> Vec<Stmt> {
    let source = format!("kazi f() -> Tupu {{\n{src}\n}}\n");
    let module = parse_tokens(&tokenize(&source).expect("tokenize")).expect("parse");
    module.functions[0].body.statements.clone()
}

#[test]
fn bitwise_binds_tighter_than_comparison() {
    let stmts = body("weka t = mask & bit == 0");
    let Stmt::Let { value, .. } = &stmts[0] else {
        panic!("expected weka");
    };
    let Expr::Binary { op, left, .. } = value else {
        panic!("expected binary");
    };
    assert_eq!(*op, BinaryOp::Eq);
    assert!(matches!(
        **left,
        Expr::Binary {
            op: BinaryOp::BitAnd,
            ..
        }
    ));
}

#[test]
fn logical_operators_still_bind_loosest() {
    let stmts = body("weka t = a | b > 1 na c == 2");
    let Stmt::Let { value, .. } = &stmts[0] else {
        panic!("expected weka");
    };
    let Expr::Binary { op, left, .. } = value else {
        panic!("expected binary");
    };
    assert_eq!(*op, BinaryOp::And);
    // `a | b > 1`  →  `(a | b) > 1`
    let Expr::Binary {
        op: BinaryOp::Gt,
        left: inner,
        ..
    } = &**left
    else {
        panic!("expected >");
    };
    assert!(matches!(
        **inner,
        Expr::Binary {
            op: BinaryOp::BitOr,
            ..
        }
    ));
}

#[test]
fn new_compound_assignments_desugar_to_binary_ops() {
    for (token, expected) in [
        ("%=", BinaryOp::Rem),
        ("&=", BinaryOp::BitAnd),
        ("|=", BinaryOp::BitOr),
        ("^=", BinaryOp::BitXor),
    ] {
        let stmts = body(&format!("x {token} 3"));
        let Stmt::Assign {
            name, op, value, ..
        } = &stmts[0]
        else {
            panic!("{token}: expected assignment");
        };
        assert_eq!(name, "x");
        assert_eq!(*op, AssignOp::Assign);
        assert!(
            matches!(value, Expr::Binary { op, left, .. } if *op == expected && matches!(**left, Expr::Ident { ref name, .. } if name == "x")),
            "{token}: {value:?}"
        );
    }
}

#[test]
fn compound_assignment_on_index_target() {
    let stmts = body("safu[r + 1] |= x");
    let Stmt::Expr {
        expr: Expr::MethodCall {
            method_name, args, ..
        },
        ..
    } = &stmts[0]
    else {
        panic!("expected ingiza call, got {:?}", stmts[0]);
    };
    assert_eq!(method_name, "ingiza");
    assert!(
        matches!(&args[1], Expr::Binary { op: BinaryOp::BitOr, left, .. }
        if matches!(**left, Expr::Index { .. }))
    );
}

#[test]
fn index_compound_assignment_rejects_calls_in_the_index() {
    let source = "kazi f() -> Tupu {\n    safu[g()] += 1\n}\n";
    let errors = parse_tokens(&tokenize(source).expect("tokenize")).expect_err("must reject");
    assert!(errors.iter().any(|d| d.code == "PAR096"), "{errors:?}");
}

#[test]
fn nested_generic_closed_by_shift_token() {
    // The lexer reads `>>` as one token; in a type it must close both generics, so `=` is still
    // seen as the start of the initializer.
    let source = "kazi f() -> Tupu {\n    weka m: Orodha<Orodha<Namba>> = orodha()\n    m.ongeza(orodha(1))\n}\n";
    let module = parse_tokens(&tokenize(source).expect("tokenize")).expect("parse");
    let Stmt::Let { ty: Some(ty), .. } = &module.functions[0].body.statements[0] else {
        panic!(
            "expected a typed let, got {:?}",
            module.functions[0].body.statements[0]
        );
    };
    assert!(
        !ty.name.contains('='),
        "type swallowed the initializer: {:?}",
        ty.name
    );
    assert_eq!(
        asili_parser::parse_value_type(&ty.name).to_string(),
        "Orodha<Orodha<Namba>>"
    );
}
