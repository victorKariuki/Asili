use asili_evaluator::{compile_module, run_bytecode_function, BytecodeProgram, Opcode, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

fn compile(source: &str) -> asili_evaluator::BytecodeProgram {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).expect("parse");
    compile_module(&module).expect("subset should lower to bytecode")
}

#[test]
fn bytecode_runs_loops_arithmetic_indexing_and_mutation() {
    let program = compile(
        r#"
        kazi jumla() -> Namba {
            weka namba: Orodha<Namba> = [1.0, 2.0, 3.0]
            namba.ingiza(1.0, 5.0)
            namba.ongeza(7.0)
            weka i: Namba = 0.0
            weka jumla: Namba = 0.0
            wakati i < namba.urefu() {
                jumla = jumla + (namba[i]?)
                i = i + 1.0
            }
            rejesha jumla
        }
        "#,
    );
    let value = run_bytecode_function(&program, "jumla", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if (n - 16.0).abs() < f64::EPSILON));
}

#[test]
fn bytecode_dispatches_user_functions_and_comparisons() {
    let program = compile(
        r#"
        kazi mara(a: Namba) -> Namba { rejesha a * 2.0 }
        kazi hesabu() -> Namba {
            ikiwa mara(3.0) >= 6.0 {
                rejesha 1.0
            }
            rejesha 0.0
        }
        "#,
    );
    let value = run_bytecode_function(&program, "hesabu", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if (n - 1.0).abs() < f64::EPSILON));
}

fn has_opcode(program: &BytecodeProgram, predicate: impl Fn(&Opcode) -> bool) -> bool {
    program
        .functions
        .iter()
        .flat_map(|function| function.code.iter())
        .any(predicate)
}

#[test]
fn numeric_list_reads_lower_to_one_unboxed_load() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka data: Orodha<Namba> = [4.0, 5.0]
            weka i: Namba = 1.0
            rejesha data[i]?
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::ListGet { .. }
    )));
    assert!(!has_opcode(&program, |op| matches!(
        op,
        Opcode::ValIndex { .. } | Opcode::Unwrap { .. }
    )));
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 5.0));
}

#[test]
fn out_of_bounds_numeric_read_propagates_tokeo_error() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka data: Orodha<Namba> = [4.0, 5.0]
            rejesha data[7.0]?
        }
        "#,
    );
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Tokeo(Err(_))), "{value:?}");
}

#[test]
fn bitwise_operators_stay_in_numeric_registers() {
    let program = compile(
        r#"
        kazi mask() -> Namba {
            weka mask: Namba = 1.0 sogeza_kushoto 3.0
            rejesha (mask au_biti 2.0) na_biti 15.0
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(op, Opcode::Shl { .. })));
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::BitOr { .. }
    )));
    assert!(!has_opcode(&program, |op| matches!(
        op,
        Opcode::ValBinary { .. }
    )));
    let value = run_bytecode_function(&program, "mask", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 10.0));
}

#[test]
fn numeric_comparisons_fuse_with_branches() {
    let program = compile(
        r#"
        kazi hesabu() -> Namba {
            weka n: Namba = 0.0
            wakati n < 10.0 {
                n += 1.0
            }
            rejesha n
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::JumpIfNot { .. }
    )));
    assert!(!has_opcode(&program, |op| matches!(op, Opcode::Cmp { .. })));
    let value = run_bytecode_function(&program, "hesabu", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 10.0));
}

#[test]
fn numeric_list_writes_use_specialized_paths() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka namba: Orodha<Namba> = [1.0, 2.0, 3.0]
            namba.ingiza(1.0, 9.0)
            namba[2] = 4
            rejesha namba[1.0]? + namba[2]?
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::ListSet { .. }
    )));
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 13.0));
}

#[test]
fn range_loops_and_recursion_work() {
    let program = compile(
        r#"
        kazi jumlisha(n: Namba) -> Namba {
            ikiwa n <= 0.0 {
                rejesha 0.0
            }
            rejesha n + jumlisha(n - 1.0)
        }
        kazi soma() -> Namba {
            weka jumla: Namba = 0
            kwa i kutoka 0 hadi 5 {
                jumla += i
            }
            rejesha jumla
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::ForStep { .. }
    )));
    let summed = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(summed, Value::Namba(n) if n == 10.0));
    let recursive = run_bytecode_function(&program, "jumlisha", vec![Value::Namba(10.0)])
        .expect("execute recursion");
    assert!(matches!(recursive, Value::Namba(n) if n == 55.0));
}

#[test]
fn logical_operators_short_circuit() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka data: Orodha<Namba> = [1.0]
            weka i: Namba = 5.0
            ikiwa i < data.urefu() na data[i]? == 1.0 {
                rejesha 1.0
            }
            weka b: Ukweli = kweli
            b = si_kweli au b
            ikiwa b {
                rejesha 2.0
            }
            rejesha 3.0
        }
        "#,
    );
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 2.0), "{value:?}");
}

#[test]
fn string_list_methods_and_callbacks_run_in_the_vm() {
    let program = compile(
        r#"
        kazi mstari(b: Orodha<Namba>) -> Neno {
            rejesha b.kwa_neno().jiunge(" ")
        }
        kazi onyesha() -> Neno {
            weka b: Orodha<Namba> = [1.0, 2.0, 3.0, 4.0]
            kwa i kutoka 0 hadi 1 {
                b.ongeza(5.0)
            }
            rejesha b.vipande(2).ramani("mstari").jiunge("|")
        }
        "#,
    );
    let value = run_bytecode_function(&program, "onyesha", vec![]).expect("execute");
    assert_eq!(value, Value::Neno("1 2|3 4|5".into()));
}

#[test]
fn unsupported_methods_fall_back_to_the_evaluator() {
    let tokens = tokenize(
        r#"
        kazi soma() -> Namba {
            weka m = 3
            kwa i kutoka 0 hadi 2 {
                m = m + i
            }
            rejesha herufi_nyingi("abc").idadi_isiyojulikana()
        }
        "#,
    )
    .expect("tokenize");
    let module = parse_tokens(&tokens).expect("parse");
    // Mixed mode: the program still compiles, `soma` is left to the tree-walker, and the
    // strict variant names it.
    let program = compile_module(&module).expect("mixed program");
    let soma = program.find_function("soma").expect("soma");
    assert!(matches!(
        soma.code.as_slice(),
        [asili_evaluator::Opcode::Interpreted { .. }]
    ));
    assert!(program.ast.is_some());
    let err = asili_evaluator::compile_module_explained(&module).expect_err("strict");
    assert!(err.contains("soma"), "{err}");
}

#[test]
fn builtin_dispatch_is_cached_for_numeric_calls() {
    let program = compile(
        r#"
        leta hisabati
        kazi sakafu_test() -> Namba {
            rejesha sakafu(3.9)
        }
        "#,
    );
    let value = run_bytecode_function(&program, "sakafu_test", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 3.0));
}
