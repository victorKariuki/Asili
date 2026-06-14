use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use asili_evaluator::{runtime, Env, Value, run_function};
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, FnContract, Module, ValueType};

#[test]
fn kamusi_iteration() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka k = kamusi_tupu()
            k.ingiza("a", 1.0)
            k.ingiza("b", 2.0)
            weka jumla = 0.0
            kwa j in k {
                weka val = j.pili()
                jumla = jumla + val
            }
            chapisha(jumla)
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let mut fns = HashMap::new();
    fns.insert("kamusi_tupu".to_string(), FnContract { params: vec![], ret: ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Namba)) });
    fns.insert("ingiza".to_string(), FnContract { params: vec![ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Namba)), ValueType::Neno, ValueType::Namba], ret: ValueType::Tupu });
    fns.insert("pili".to_string(), FnContract { params: vec![ValueType::Jozi(Box::new(ValueType::Neno), Box::new(ValueType::Namba))], ret: ValueType::Namba });
    fns.insert("chapisha".to_string(), FnContract { params: vec![ValueType::Namba], ret: ValueType::Tupu });
    
    semantic_check_with_env(&module, true, fns, HashMap::new()).expect("semantic");

    // Mock chapisha
    let output = Arc::new(Mutex::new(0.0));
    let output_clone = output.clone();
    let chapisha = Box::new(move |args: &[Value]| {
        if let Some(Value::Namba(n)) = args.get(0) {
            let mut o = output_clone.lock().unwrap();
            *o = *n;
        }
        Ok(Value::Tupu)
    });

    let mut builtins = asili_evaluator::builtins::builtins();
    builtins.insert("chapisha".to_string(), chapisha);
    
    let result = asili_evaluator::run_function_with_builtins(
        &module,
        "kuu",
        vec![Value::Orodha(vec![])],
        builtins
    ).expect("eval");
    
    let final_output = *output.lock().unwrap();
    assert_eq!(final_output, 3.0);
    assert_eq!(result, Value::Tupu);
}
