//! Built-in functions for random numbers, dates and times, files, directories, paths, the
//! environment and other programs: each snippet against a hand-checked expectation, run as
//! native code.

use asili_evaluator::{run_function, Value};
use asili_lexer::tokenize;
use asili_parser::parse_tokens;

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
    let source = format!(
        "leta hisabati\nleta majira\nleta faili\nleta mfumo\nkazi t() -> Neno {{\n{body}\n}}\n"
    );
    assert_eq!(run(&source), expected, "\n{source}");
}

#[test]
fn seeded_random_numbers_repeat() {
    check(
        r#"nasibu_mbegu(42)
weka a = [nasibu(), nasibu(), nasibu_kamili(1, 6)]
nasibu_mbegu(42)
weka b = [nasibu(), nasibu(), nasibu_kamili(1, 6)]
rejesha (a.jiunge(",") == b.jiunge(",")) kama Neno"#,
        "kweli",
    );
    // Whole numbers within the bounds, both included.
    check(
        r#"nasibu_mbegu(7)
weka sawa = kweli
weka juu = 0
weka chini = 10
kwa i kutoka 0 hadi 2000 {
    weka n = nasibu_kamili(3, 5)
    ikiwa n > juu { juu = n }
    ikiwa n < chini { chini = n }
    ikiwa n != sakafu(n) { sawa = si_kweli }
}
rejesha (sawa kama Neno) + " " + (chini kama Neno) + " " + (juu kama Neno)"#,
        "kweli 3 5",
    );
    // A shuffle is a permutation; picking from an empty list gives Hamna.
    check(
        r#"nasibu_mbegu(1)
weka c = changanya([1, 2, 3, 4, 5, 6, 7, 8])
rejesha c.panga().jiunge(",") + " " + (chagua_nasibu([]).ni_tupu() kama Neno)"#,
        "1,2,3,4,5,6,7,8 kweli",
    );
}

#[test]
fn dates_and_times() {
    check(
        r#"rejesha kwa_iso(kutoka_sekunde(0))"#,
        "1970-01-01T00:00:00Z",
    );
    check(
        r#"rejesha kwa_iso(kutoka_sekunde(1709209815.25))"#,
        "2024-02-29T12:30:15.250Z",
    );
    // Before 1970: no longer a crash.
    check(
        r#"rejesha umbiza(kutoka_sekunde(-5))"#,
        "1969-12-31 23:59:55",
    );
    check(
        r#"rejesha umbiza_eneo(kutoka_sekunde(0), 180)"#,
        "1970-01-01 03:00:00+03:00",
    );
    check(
        r#"rejesha kwa_iso(kutoka_iso("2026-10-09T15:30:00+03:00").angu(kutoka_sekunde(0)))"#,
        "2026-10-09T12:30:00Z",
    );
    check(
        r#"rejesha kutoka_iso("2026-02-30").ni_kosa() kama Neno"#,
        "kweli",
    );
    check(
        r#"weka t = kutoka_tarehe(2024, 2, 29).angu(kutoka_sekunde(0))
weka p = tarehe(t)
rejesha (p.pata("mwezi").angu(0) kama Neno) + "/" + (p.pata("siku").angu(0) kama Neno) + " wiki " + (p.pata("siku_ya_wiki").angu(0) kama Neno) + " mwaka " + (p.pata("siku_ya_mwaka").angu(0) kama Neno)"#,
        "2/29 wiki 4 mwaka 60",
    );
    check(
        r#"rejesha kutoka_tarehe(2023, 2, 29).ni_kosa() kama Neno"#,
        "kweli",
    );
    check(
        r#"weka a = kipima_muda()
weka b = kipima_muda()
rejesha (b >= a) kama Neno"#,
        "kweli",
    );
}

#[test]
fn directories_and_paths() {
    let dir = std::env::temp_dir().join(format!("asili-saraka-{}", std::process::id()));
    let d = dir.to_string_lossy().replace('\\', "/");
    check(
        &format!(
            r#"weka d = "{d}"
unda_saraka(njia_unganisha(d, "ndani/zaidi")).ni_sawa()
andika_faili(njia_unganisha(d, "b.txt"), "habari").ni_sawa()
andika_faili(njia_unganisha(d, "a.txt"), "x").ni_sawa()
weka n = nakili(njia_unganisha(d, "b.txt"), njia_unganisha(d, "c.txt")).angu(0)
badili_jina(njia_unganisha(d, "a.txt"), njia_unganisha(d, "z.txt")).ni_sawa()
weka majina = orodha_saraka(d).angu([]).jiunge(",")
weka aina = (ni_saraka(njia_unganisha(d, "ndani")) kama Neno) + (ni_faili(njia_unganisha(d, "ndani")) kama Neno)
futa_saraka(d).ni_sawa()
rejesha majina + " " + (n kama Neno) + " " + aina + " " + (vipo(d) kama Neno)"#
        ),
        "b.txt,c.txt,ndani,z.txt 6 kwelisi_kweli si_kweli",
    );
    check(
        r#"rejesha orodha_saraka("/hakuna/saraka/hii").ni_kosa() kama Neno"#,
        "kweli",
    );
    check(
        r#"rejesha njia_jina("/a/b/faili.tar.gz").angu("")"#,
        "faili.tar.gz",
    );
    check(
        r#"rejesha njia_kiendelezi("/a/b/faili.tar.gz").angu("")"#,
        "gz",
    );
    check(r#"rejesha njia_mzazi("/a/b/faili.txt").angu("")"#, "/a/b");
    check(
        r#"rejesha njia_mzazi("faili.txt").ni_tupu() kama Neno"#,
        "kweli",
    );
    check(
        r#"rejesha njia_kiendelezi("Makefile").ni_tupu() kama Neno"#,
        "kweli",
    );
}

#[test]
fn environment_and_programs() {
    check(
        r#"weka_env("ASILI_JARIBIO_ENV", "ndiyo")
rejesha pata_env("ASILI_JARIBIO_ENV").angu("")"#,
        "ndiyo",
    );
    if cfg!(unix) {
        check(
            r#"rejesha endesha("echo", ["habari", "dunia"]).angu("")"#,
            "habari dunia\n",
        );
        check(
            r#"rejesha endesha("sh", ["-c", "echo shida >&2; exit 3"]).kosa()"#,
            "sh imeshindwa (msimbo 3): shida",
        );
        check(
            r#"rejesha endesha("hakuna-programu-hii", []).ni_kosa() kama Neno"#,
            "kweli",
        );
    }
}
