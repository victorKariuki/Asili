use asili_evaluator::Value;
use asili_lexer::tokenize;
use asili_parser::{parse_tokens, semantic_check_with_env, FnContract, ValueType};
use std::collections::HashMap;

#[test]
fn kamusi_iteration() {
    let src = r#"
        kazi kuu(hoja: Orodha<Neno>) -> Tupu {
            weka k = kamusi_tupu()
            k.ingiza("a", 1.0)
            k.ingiza("b", 2.0)
            weka jumla = 0.0
            kwa j katika k {
                weka val = j.pili()
                jumla = jumla + val
            }
            chapisha(jumla)
        }
        kazi jumla_ya_kamusi() -> Namba {
            weka k = kamusi_tupu()
            k.ingiza("a", 1.0)
            k.ingiza("b", 2.0)
            weka jumla = 0.0
            kwa j katika k {
                jumla = jumla + j.pili()
            }
            rejesha jumla
        }
    "#;
    let toks = tokenize(src).expect("tokenize");
    let module = parse_tokens(&toks).expect("parse");
    let mut fns = HashMap::new();
    fns.insert(
        "kamusi_tupu".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Namba)),
        },
    );
    fns.insert(
        "ingiza".to_string(),
        FnContract {
            params: vec![
                ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Namba)),
                ValueType::Neno,
                ValueType::Namba,
            ],
            ret: ValueType::Tupu,
        },
    );
    fns.insert(
        "pili".to_string(),
        FnContract {
            params: vec![ValueType::Jozi(
                Box::new(ValueType::Neno),
                Box::new(ValueType::Namba),
            )],
            ret: ValueType::Namba,
        },
    );
    fns.insert(
        "chapisha".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tupu,
        },
    );

    semantic_check_with_env(&module, true, fns, HashMap::new()).expect("semantic");

    let total = asili_evaluator::run_function(&module, "jumla_ya_kamusi", vec![]).expect("run");
    assert_eq!(total, Value::Namba(3.0));
}
