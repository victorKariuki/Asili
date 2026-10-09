//! Built-in methods of text, lists, maps and sets: each snippet's result (shown as text) against
//! a hand-checked expectation. Everything runs as native code, through its host. List methods
//! are checked on typed numeric lists (`ListMethod`, run on the list's own storage) and on
//! generic lists, which must give the same answers.

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

/// The text of `kazi t() -> Neno` in `source`, or the error.
fn run(source: &str) -> String {
    let tokens = tokenize(source).expect("tokenize");
    let module = parse_tokens(&tokens).expect("parse");
    match run_function(&module, "t", vec![]) {
        Ok(Value::Neno(s)) => s.to_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("kosa: {e}"),
    }
}

fn check(body: &str, expected: &str) {
    let source =
        format!("kazi moja(x: Namba) -> Namba {{ rejesha x }}\nkazi t() -> Neno {{\n{body}\n}}\n");
    assert_eq!(run(&source), expected, "\n{source}");
}

/// `expr` (using list `a`) on `a` as a typed numeric list and as a generic list holding the
/// same numbers: both must give `expected`.
fn check_list(items: &str, expr: &str, expected: &str) {
    // Typed: a `Orodha<Namba>` local lives in a numeric list register.
    check(
        &format!("weka a: Orodha<Namba> = [{items}]\nrejesha {expr}"),
        expected,
    );
    // Generic: the same numbers, made through a generic list operation.
    check(
        &format!("weka b = [{items}]\nweka a = b.ramani(\"moja\")\nrejesha {expr}"),
        expected,
    );
}

#[test]
fn text_is_sliced_and_searched_by_character() {
    // `é` is two bytes: positions count characters, as `urefu` does.
    check(r#"rejesha "café au lait".kata(0, 4)"#, "café");
    check(r#"rejesha "café au lait".kata(3, 6)"#, "é a");
    check(r#"rejesha "naïve".kata(2)"#, "ïve");
    check(r#"rejesha "abc".kata(2, 99)"#, "c");
    check(r#"rejesha "abc".kata(5, 9)"#, "");
    check(r#"rejesha "abc".kata(2, 1)"#, "");
    check(r#"rejesha "héllo".kata(1, 1)"#, "");
    check(
        r#"rejesha "café au lait".tafuta("au").angu(-1) kama Neno"#,
        "5",
    );
    check(
        r#"rejesha "👋🏽 habari".tafuta("habari").angu(-1) kama Neno"#,
        "2",
    );
    check(r#"rejesha "👋🏽 habari".kata(0, 1)"#, "👋🏽");
    check(r#"rejesha "abc".tafuta("x").ni_tupu() kama Neno"#, "kweli");
}

#[test]
fn escapes() {
    check(
        r#"rejesha ("a\rb".urefu() kama Neno) + (("\u{e9}" == "é") kama Neno)"#,
        "3kweli",
    );
    check(r#"rejesha "x\0y".misimbo().jiunge(",")"#, "120,0,121");
    check(r#"rejesha ('a' == '\r') kama Neno"#, "si_kweli");
}

#[test]
fn text_methods() {
    check(r#"rejesha "  habari \n".safisha()"#, "habari");
    check(r#"rejesha "  habari ".safisha_mwanzo()"#, "habari ");
    check(r#"rejesha "  habari ".safisha_mwisho()"#, "  habari");
    check(r#"rejesha "7".jaza_kushoto(3, "0")"#, "007");
    check(r#"rejesha "ab".jaza_kulia(4)"#, "ab  ");
    check(r#"rejesha "abcdef".jaza_kushoto(3, "0")"#, "abcdef");
    check(r#"rejesha "é".jaza_kushoto(3, "-")"#, "--é");
    check(
        r#"rejesha "x".jaza_kushoto(3, "ab")"#,
        "kosa: aina: jaza_kushoto inahitaji herufi moja ya kujaza",
    );
    check(
        r#"rejesha "{} ana miaka {}, {{sawa}} {}".jaza(["Amara", 30, kweli])"#,
        "Amara ana miaka 30, {sawa} kweli",
    );
    check(
        r#"rejesha "{} {}".jaza([1])"#,
        "kosa: aina: jaza: nafasi {} ni nyingi kuliko thamani",
    );
    check(
        r#"rejesha "{}".jaza([1, 2])"#,
        "kosa: aina: jaza: thamani ni nyingi kuliko nafasi {}",
    );
    check(r#"rejesha "café".herufi().jiunge("|")"#, "c|a|f|é");
    check(r#"rejesha "abc".herufi().jiunge("|")"#, "a|b|c");
    check(r#"rejesha "a\nb\r\nc".mistari().jiunge(",")"#, "a,b,c");
    check(r#"rejesha "café".geuza()"#, "éfac");
    check(r#"rejesha "abc".geuza()"#, "cba");
    check(r#"rejesha "Aé".misimbo().jiunge(",")"#, "65,233");
    check(
        r#"rejesha (" 2.5 ".kwa_namba().angu(0) * 2) kama Neno"#,
        "5",
    );
    check(r#"rejesha "abc".kwa_namba().ni_kosa() kama Neno"#, "kweli");
}

#[test]
fn list_methods_on_numbers() {
    check_list("3, 1, 2", r#"a.panga().jiunge(",")"#, "1,2,3");
    check_list(
        "3, -1, 2.5, 300, -70000",
        r#"a.panga().jiunge(",")"#,
        "-70000,-1,2.5,3,300",
    );
    check_list("1, 2, 3", r#"a.geuza().jiunge(",")"#, "3,2,1");
    check_list("1, 2, 3", r#"a.ina(2) kama Neno"#, "kweli");
    check_list("1, 2, 3", r#"a.ina(9) kama Neno"#, "si_kweli");
    check_list("1, 2, 3", r#"a.ina("2") kama Neno"#, "si_kweli");
    check_list("5, 6, 7", r#"a.tafuta(7).angu(-1) kama Neno"#, "2");
    check_list("5, 6, 7", r#"a.tafuta(1).ni_tupu() kama Neno"#, "kweli");
    check_list("5, 6, 7, 8", r#"a.kata(1, 3).jiunge(",")"#, "6,7");
    check_list("5, 6, 7, 8", r#"a.kata(2).jiunge(",")"#, "7,8");
    check_list("5, 6, 7, 8", r#"a.kata(9).urefu() kama Neno"#, "0");
    check_list("4, 9, 2", r#"a.kubwa().angu(0) kama Neno"#, "9");
    check_list("4, 9, 2", r#"a.ndogo().angu(0) kama Neno"#, "2");
    check_list("1.5, 2, 3", r#"a.jumla() kama Neno"#, "6.5");
    check_list("3, 1, 3, 2, 1", r#"a.kipekee().jiunge(",")"#, "3,1,2");
    check_list("7, 8", r#"a.kwanza().angu(0) kama Neno"#, "7");
    check_list("7, 8", r#"a.mwisho().angu(0) kama Neno"#, "8");
    check_list("7, 8", r#"a.tupu() kama Neno"#, "si_kweli");
    check_list("7, 8", r#"a.pata(1).angu(0) kama Neno"#, "8");
    // A sorted list is a list again: methods chain on it.
    check_list("9, 4, 7", r#"a.panga().kata(0, 2).jumla() kama Neno"#, "11");
}

#[test]
fn empty_lists_and_signed_zero() {
    check(
        "weka a: Orodha<Namba> = []\nrejesha (a.kubwa().ni_tupu() kama Neno) + \" \" + (a.jumla() kama Neno) + \" \" + (a.tupu() kama Neno)",
        "kweli 0 kweli",
    );
    // 0 and -0 are equal (`ina`), sort as equal in their original order, and are different
    // values to `kipekee` (as a Seti keeps them).
    check_list("0, -0", r#"(1 / a.panga()[0]) kama Neno"#, "Ukomo");
    check_list("-0, 0", r#"(1 / a.panga()[0]) kama Neno"#, "-Ukomo");
    check_list("0, -0", r#"a.kipekee().urefu() kama Neno"#, "2");
    check_list("-0", r#"a.ina(0) kama Neno"#, "kweli");
}

#[test]
fn list_methods_on_other_values() {
    check(
        r#"rejesha ["pera", "embe", "ndizi"].panga().jiunge(",")"#,
        "embe,ndizi,pera",
    );
    check(
        r#"rejesha [1, "a"].panga().jiunge(",")"#,
        "kosa: aina: kupanga kunahitaji Namba zote, Neno zote, Herufi zote au Ukweli zote",
    );
    check(r#"rejesha ["a", "b", "a"].kipekee().jiunge(",")"#, "a,b");
    check(r#"rejesha ["x", "y"].ina("y") kama Neno"#, "kweli");
    check(r#"rejesha ["x", "y"].kubwa().angu("") "#, "y");
    check(
        r#"rejesha ["a", "b"].jumla() kama Neno"#,
        "kosa: aina: jumla inahitaji Orodha ya Namba",
    );
    check(
        r#"weka a = [1, 2]
a.ongeza_zote([3, 4])
weka n = a.urefu()
a.futa_zote()
rejesha (n kama Neno) + " " + (a.urefu() kama Neno)"#,
        "4 0",
    );
    // Clearing a shared list leaves its copies alone.
    check(
        r#"weka a = ["p", "q"]
weka b = a
a.futa_zote()
rejesha (a.urefu() kama Neno) + " " + (b.urefu() kama Neno)"#,
        "0 2",
    );
}

#[test]
fn sorting_by_a_key() {
    let source = r#"
        kazi urefu_wake(s: Neno) -> Namba { rejesha s.urefu() }
        kazi t() -> Neno {
            rejesha ["ndizi", "pera", "embe", "papai"].panga_kwa("urefu_wake").jiunge(",")
        }
    "#;
    // Stable: equal keys keep their order.
    assert_eq!(run(source), "pera,embe,ndizi,papai");
}

#[test]
fn map_and_set_methods() {
    check(
        r#"weka m = kamusi_tupu()
m.ingiza("a", 1)
m.ingiza("b", 2)
weka kale = m.ondoa("a").angu(0)
weka hakuna = m.ondoa("x").ni_tupu()
rejesha (kale kama Neno) + " " + (hakuna kama Neno) + " " + (m.idadi() kama Neno) + " " + (m.thamani().jumla() kama Neno)"#,
        "1 kweli 1 2",
    );
    check(
        r#"weka m = kamusi_tupu()
m.ingiza("x", 5)
weka p = m.vipengele()[0]
rejesha p.kwanza() + "=" + (p.pili() kama Neno)"#,
        "x=5",
    );
    check(
        r#"weka m = kamusi_tupu()
m.ingiza("x", 5)
weka n = m
m.futa_zote()
rejesha (m.idadi() kama Neno) + " " + (n.idadi() kama Neno)"#,
        "0 1",
    );
    check(
        r#"weka a = seti(1, 2, 3)
weka b = seti(2, 3, 4)
weka u = a.muungano(b).orodha().panga().jiunge(",")
weka k = a.makutano(b).orodha().panga().jiunge(",")
weka t2 = a.tofauti(b).orodha().panga().jiunge(",")
rejesha u + " " + k + " " + t2 + " " + (seti(2).ni_sehemu_ya(a) kama Neno) + " " + (a.ni_sehemu_ya(b) kama Neno)"#,
        "1,2,3,4 2,3 1 kweli si_kweli",
    );
}
