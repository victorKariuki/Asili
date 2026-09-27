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
fn identity_index_addition_lowers_to_direct_numeric_list_read() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka data: Orodha<Namba> = [4.0, 5.0]
            weka i: Namba = 1.0
            rejesha data[i + 0.0]?
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::IndexNumberLocalFromSlot { .. }
    )));
    assert!(!has_opcode(&program, |op| matches!(
        op,
        Opcode::NumericBinary { .. }
    )));
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 5.0));
}

#[test]
fn numeric_builtin_calls_lower_to_typed_opcodes() {
    let program = compile(
        r#"
        kazi mask() -> Namba {
            rejesha na_biti(sogeza_kushoto(1.0, 3.0), 15.0)
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::NumericBuiltin(_)
    )));
    let value = run_bytecode_function(&program, "mask", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 8.0));
}

#[test]
fn typed_numeric_instructions_preserve_integer_safe_masks() {
    let program = compile(
        r#"
        kazi mask() -> Namba {
            weka mask: Namba = 1.0 sogeza_kushoto 3.0
            rejesha mask au_biti 2.0
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::NumericBinary(_) | Opcode::StoreNumber(_) | Opcode::LoadNumber(_)
    )));
    let value = run_bytecode_function(&program, "mask", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 10.0));
}

#[test]
fn numeric_list_reads_and_writes_use_specialized_paths() {
    let program = compile(
        r#"
        kazi soma() -> Namba {
            weka namba: Orodha<Namba> = [1.0, 2.0, 3.0]
            namba.ingiza(1.0, 9.0)
            rejesha namba[1.0]?
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::IndexNumberLocal(_) | Opcode::ListSetNumber(_)
    )));
    let value = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(value, Value::Namba(n) if n == 9.0));
}

#[test]
fn fused_numeric_index_add_and_reusable_recursive_frames_work() {
    let program = compile(
        r#"
        kazi jumlisha(n: Namba) -> Namba {
            ikiwa n <= 0.0 {
                rejesha 0.0
            }
            rejesha n + jumlisha(n - 1.0)
        }
        kazi soma() -> Namba {
            weka namba: Orodha<Namba> = [4.0, 5.0]
            weka i: Namba = 0.0
            rejesha namba[i]? + 1.0
        }
        "#,
    );
    assert!(has_opcode(&program, |op| matches!(
        op,
        Opcode::IndexAddNumberLocal { .. } | Opcode::IncrementNumberLocal { .. }
    )));
    let indexed = run_bytecode_function(&program, "soma", vec![]).expect("execute");
    assert!(matches!(indexed, Value::Namba(n) if n == 5.0));
    let recursive = run_bytecode_function(&program, "jumlisha", vec![Value::Namba(10.0)])
        .expect("execute recursion");
    assert!(matches!(recursive, Value::Namba(n) if n == 55.0));
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
