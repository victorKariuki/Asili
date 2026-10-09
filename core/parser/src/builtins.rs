//! Builtin module export tables (msingi, mfumo, majira, matumizi, faili, hisabati, ruwaza,
//! usimbaji, ...). Resolved without disk I/O.

use crate::{FnContract, ValueType};
use std::collections::HashMap;

/// Builtin modules whose exports are *not* ambient: they must be brought in with `leta` (the
/// managed-memory wrapper is opt-in by design — see docs/design/kasha-gc-design.md).
pub const OPT_IN_MODULES: &[&str] = &["kasha_gc"];

/// The builtin modules whose exports are in scope without `leta`.
pub fn ambient_module_names() -> impl Iterator<Item = &'static str> {
    BUILTIN_MODULE_NAMES
        .iter()
        .copied()
        .filter(|name| !OPT_IN_MODULES.contains(name))
}

pub const BUILTIN_MODULE_NAMES: &[&str] = &[
    "msingi", "mfumo", "majira", "matumizi", "faili", "hisabati", "runtime", "syscall", "kiungo",
    "sambamba", "kasha_gc", "ruwaza", "usimbaji",
];

/// Export table: functions and constants for a builtin module.
#[derive(Clone, Debug, Default)]
pub struct BuiltinExportTable {
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
}

fn namba_namba_namba() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba, ValueType::Namba],
        ret: ValueType::Namba,
    }
}

fn namba_namba_tokeo_namba() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba, ValueType::Namba],
        ret: ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
    }
}

fn namba_namba() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba],
        ret: ValueType::Namba,
    }
}

fn namba_tokeo_namba() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba],
        ret: ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
    }
}

fn namba_namba_tokeo_namba_opt() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba, ValueType::Namba],
        ret: ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
    }
}

fn namba_namba_namba_namba() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba, ValueType::Namba, ValueType::Namba],
        ret: ValueType::Namba,
    }
}

fn namba_ukweli() -> FnContract {
    FnContract {
        params: vec![ValueType::Namba],
        ret: ValueType::Ukweli,
    }
}

fn ret_namba() -> FnContract {
    FnContract {
        params: vec![],
        ret: ValueType::Namba,
    }
}

/// Prelude: foundation types and constructors. Always in scope.
/// Add `name(params) -> ret` to an export table.
fn contract(
    functions: &mut HashMap<String, FnContract>,
    name: &str,
    params: Vec<ValueType>,
    ret: ValueType,
) {
    functions.insert(name.to_string(), FnContract { params, ret });
}

pub fn msingi_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "orodha".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Orodha(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "orodha_rudia".to_string(),
        FnContract {
            params: vec![ValueType::Unknown, ValueType::Namba],
            ret: ValueType::Orodha(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "kamusi".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Kamusi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "kamusi_tupu".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Kamusi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "jozi".to_string(),
        FnContract {
            params: vec![ValueType::Unknown, ValueType::Unknown],
            ret: ValueType::Jozi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "tokeo".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::Tokeo(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "kosa".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Unknown), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "chaguo".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::Chaguo(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "kumbukumbu_unda".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::Kumbukumbu(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "seti".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Seti(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "seti_tupu".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Seti(Box::new(ValueType::Unknown)),
        },
    );
    let mut constants = HashMap::new();
    constants.insert("KWELI".to_string(), ValueType::Ukweli);
    constants.insert("SIYO_KWELI".to_string(), ValueType::Ukweli);
    constants.insert("TUPU".to_string(), ValueType::Tupu);
    BuiltinExportTable {
        functions,
        constants,
    }
}

pub fn hisabati_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    contract(
        &mut functions,
        "nasibu_kamili",
        vec![ValueType::Namba, ValueType::Namba],
        ValueType::Namba,
    );
    contract(
        &mut functions,
        "nasibu_mbegu",
        vec![ValueType::Namba],
        ValueType::Tupu,
    );
    contract(
        &mut functions,
        "changanya",
        vec![ValueType::Orodha(Box::new(ValueType::Unknown))],
        ValueType::Orodha(Box::new(ValueType::Unknown)),
    );
    contract(
        &mut functions,
        "chagua_nasibu",
        vec![ValueType::Orodha(Box::new(ValueType::Unknown))],
        ValueType::Chaguo(Box::new(ValueType::Unknown)),
    );
    functions.insert(
        "namba_kuu_kutoka".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::NambaKuu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "namba_sahihi_kutoka".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::NambaSahihi), Box::new(ValueType::Neno)),
        },
    );
    functions.insert("jumla".to_string(), namba_namba_namba());
    functions.insert("tofauti".to_string(), namba_namba_namba());
    functions.insert("zao".to_string(), namba_namba_namba());
    functions.insert("gawio".to_string(), namba_namba_tokeo_namba());
    functions.insert("duara".to_string(), namba_namba());
    functions.insert("absolute".to_string(), namba_namba());
    functions.insert("kipeo".to_string(), namba_namba_tokeo_namba_opt());
    functions.insert("mizizi".to_string(), namba_tokeo_namba());
    functions.insert("abs".to_string(), namba_namba());
    functions.insert("ishara".to_string(), namba_namba());
    functions.insert("upeo".to_string(), namba_namba_tokeo_namba_opt());
    functions.insert("kipeuo2".to_string(), namba_tokeo_namba());
    functions.insert("kipeuo3".to_string(), namba_namba());
    functions.insert("haipot".to_string(), namba_namba_namba());
    functions.insert("upeo_wa_e".to_string(), namba_namba());
    functions.insert("expm1".to_string(), namba_namba());
    functions.insert("sakafu".to_string(), namba_namba());
    functions.insert("dari".to_string(), namba_namba());
    functions.insert("duara_maeneo".to_string(), namba_namba_namba());
    functions.insert("punguza".to_string(), namba_namba());
    functions.insert("kubwa".to_string(), namba_namba_namba());
    functions.insert("ndogo".to_string(), namba_namba_namba());
    functions.insert("kikwazo".to_string(), namba_namba_namba_namba());
    functions.insert("kwenda_radiani".to_string(), namba_namba());
    functions.insert("kwenda_nyuzi".to_string(), namba_namba());
    functions.insert("mzizi".to_string(), namba_namba_tokeo_namba());
    functions.insert("faktoriali".to_string(), namba_tokeo_namba());
    functions.insert("baki".to_string(), namba_namba_tokeo_namba());
    functions.insert("logi".to_string(), namba_tokeo_namba());
    functions.insert("logi10".to_string(), namba_tokeo_namba());
    functions.insert("logi2".to_string(), namba_tokeo_namba());
    functions.insert("logi1p".to_string(), namba_tokeo_namba());
    functions.insert("asini".to_string(), namba_tokeo_namba());
    functions.insert("akosini".to_string(), namba_tokeo_namba());
    functions.insert("atanjenti".to_string(), namba_tokeo_namba());
    functions.insert("asini_h".to_string(), namba_tokeo_namba());
    functions.insert("akosini_h".to_string(), namba_tokeo_namba());
    functions.insert("atanjenti_h".to_string(), namba_tokeo_namba());
    functions.insert("sini".to_string(), namba_namba());
    functions.insert("kosini".to_string(), namba_namba());
    functions.insert("tanjenti".to_string(), namba_namba());
    functions.insert("atanjenti2".to_string(), namba_namba_namba());
    functions.insert("sini_h".to_string(), namba_namba());
    functions.insert("kosini_h".to_string(), namba_namba());
    functions.insert("tanjenti_h".to_string(), namba_namba());
    functions.insert("ni_namba".to_string(), namba_ukweli());
    functions.insert("si_namba".to_string(), namba_ukweli());
    functions.insert("ni_ukomo".to_string(), namba_ukweli());
    functions.insert("nasibu".to_string(), ret_namba());
    functions.insert("nasibu_chini".to_string(), namba_namba_namba());

    let mut constants = HashMap::new();
    constants.insert("Ukomo".to_string(), ValueType::Namba);
    constants.insert("Siyo_Namba".to_string(), ValueType::Namba);
    constants.insert("PI".to_string(), ValueType::Namba);
    constants.insert("E".to_string(), ValueType::Namba);
    constants.insert("PHI".to_string(), ValueType::Namba);
    constants.insert("TAU".to_string(), ValueType::Namba);
    constants.insert("LN10".to_string(), ValueType::Namba);
    constants.insert("LN2".to_string(), ValueType::Namba);
    constants.insert("LOG10E".to_string(), ValueType::Namba);
    constants.insert("LOG2E".to_string(), ValueType::Namba);
    constants.insert("KIPEUO1_2".to_string(), ValueType::Namba);
    constants.insert("KIPEUO2".to_string(), ValueType::Namba);
    constants.insert("KIPEUO3".to_string(), ValueType::Namba);
    constants.insert("KIPEUO5".to_string(), ValueType::Namba);
    constants.insert("EPSILON".to_string(), ValueType::Namba);
    constants.insert("INF".to_string(), ValueType::Namba);
    constants.insert("NAN".to_string(), ValueType::Namba);

    BuiltinExportTable {
        functions,
        constants,
    }
}

/// System: env, args, exit. Requires `leta mfumo`.
pub fn mfumo_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    contract(
        &mut functions,
        "weka_env",
        vec![ValueType::Neno, ValueType::Neno],
        ValueType::Tupu,
    );
    contract(
        &mut functions,
        "http_pata",
        vec![ValueType::Neno],
        tokeo_neno(ValueType::Neno),
    );
    contract(
        &mut functions,
        "http_tuma",
        vec![ValueType::Neno, ValueType::Neno, ValueType::Neno],
        tokeo_neno(ValueType::Neno),
    );
    // The response is a `JibuHttp { hali, vichwa, mwili }` struct, Unknown at the signature level
    // like the server's `OmbiHttp`/`JibuHttp` (see `mkondo_tumikia_http`).
    contract(
        &mut functions,
        "http_ombi",
        vec![
            ValueType::Neno,
            ValueType::Neno,
            ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
            ValueType::Neno,
        ],
        tokeo_neno(ValueType::Unknown),
    );
    contract(
        &mut functions,
        "endesha",
        vec![
            ValueType::Neno,
            ValueType::Orodha(Box::new(ValueType::Neno)),
        ],
        ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
    );
    functions.insert(
        "vigezo".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Orodha(Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "pata_env".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Chaguo(Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "toka".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "mlinzi_anza".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "mlinzi_lisha".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "kikomo_kumbukumbu".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "sikiliza_ishara".to_string(),
        FnContract {
            params: vec![ValueType::Namba, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "rejesha_ishara".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "mkondo_unganisha".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Mkondo), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "mkondo_sikiliza".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(
                Box::new(ValueType::MkondoSikilizaji),
                Box::new(ValueType::Neno),
            ),
        },
    );
    functions.insert(
        "mkondo_tumikia".to_string(),
        FnContract {
            params: vec![
                ValueType::MkondoSikilizaji,
                ValueType::Neno,
                ValueType::Namba,
                ValueType::Chaguo(Box::new(ValueType::TlsUsanidi)),
            ],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "mkondo_tumikia_http".to_string(),
        FnContract {
            // ombi/jibu are Value::Struct("OmbiHttp"/"JibuHttp", ...) at runtime — Unknown here
            // since this codebase's FnContract has no way to express "a struct with these named
            // fields," the same reflection-friendly-but-untyped-at-the-signature-level tradeoff
            // the JSON codec's kutoka_json already accepts (see json-codec-design.md).
            params: vec![
                ValueType::MkondoSikilizaji,
                ValueType::Neno,
                ValueType::Namba,
                ValueType::Chaguo(Box::new(ValueType::TlsUsanidi)),
            ],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "tls_sanidi".to_string(),
        FnContract {
            params: vec![ValueType::Neno, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::TlsUsanidi), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "kwa_json".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "kutoka_json".to_string(),
        // Declared as Kamusi<Neno, Unknown> — the codec's runtime output shape actually varies
        // (a JSON array decodes as Orodha, a scalar as Namba/Neno/etc.), but a builtin's return
        // type here is one static type, and a JSON object (the common "parse a response body"
        // case) is the shape whose fields need `.pata(...)` to be statically callable at all.
        // Other shapes still work at runtime; only their static method/index calls need a cast.
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(
                Box::new(ValueType::Kamusi(
                    Box::new(ValueType::Neno),
                    Box::new(ValueType::Unknown),
                )),
                Box::new(ValueType::Neno),
            ),
        },
    );
    let mut constants = HashMap::new();
    constants.insert("TOLEO".to_string(), ValueType::Neno);
    constants.insert("JINA_OS".to_string(), ValueType::Neno);
    BuiltinExportTable {
        functions,
        constants,
    }
}

/// Time: seconds since epoch, sleep, format. Requires `leta majira`.
pub fn majira_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    contract(
        &mut functions,
        "kwa_iso",
        vec![ValueType::Wakati],
        ValueType::Neno,
    );
    contract(
        &mut functions,
        "umbiza_eneo",
        vec![ValueType::Wakati, ValueType::Namba],
        ValueType::Neno,
    );
    contract(
        &mut functions,
        "kutoka_iso",
        vec![ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Wakati), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "kutoka_tarehe",
        vec![ValueType::Namba, ValueType::Namba, ValueType::Namba],
        ValueType::Tokeo(Box::new(ValueType::Wakati), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "tarehe",
        vec![ValueType::Wakati],
        ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Namba)),
    );
    contract(&mut functions, "kipima_muda", vec![], ValueType::Namba);
    functions.insert(
        "majira".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Namba,
        },
    );
    functions.insert(
        "sasa".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Wakati,
        },
    );
    functions.insert(
        "sekunde".to_string(),
        FnContract {
            params: vec![ValueType::Wakati],
            ret: ValueType::Namba,
        },
    );
    functions.insert(
        "kutoka_sekunde".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Wakati,
        },
    );
    functions.insert(
        "umbiza".to_string(),
        FnContract {
            params: vec![ValueType::Wakati],
            ret: ValueType::Neno,
        },
    );
    functions.insert(
        "lala".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tupu,
        },
    );
    let mut constants = HashMap::new();
    constants.insert("SEKUNDE_KWA_SIKU".to_string(), ValueType::Namba);
    constants.insert("MWANZO_WA_ZAMANI".to_string(), ValueType::Namba);
    BuiltinExportTable {
        functions,
        constants,
    }
}

/// I/O: print, stderr, prompt. Requires `leta matumizi`.
pub fn matumizi_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "chapisha".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "onyo".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "makosa".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "paparika".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tupu,
        },
    );
    functions.insert(
        "omba".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Neno,
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// File system: read, write, append, exists, delete, size. Requires `leta faili`.
pub fn faili_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    contract(
        &mut functions,
        "orodha_saraka",
        vec![ValueType::Neno],
        ValueType::Tokeo(
            Box::new(ValueType::Orodha(Box::new(ValueType::Neno))),
            Box::new(ValueType::Neno),
        ),
    );
    contract(
        &mut functions,
        "unda_saraka",
        vec![ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "futa_saraka",
        vec![ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "badili_jina",
        vec![ValueType::Neno, ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "nakili",
        vec![ValueType::Neno, ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "ni_saraka",
        vec![ValueType::Neno],
        ValueType::Ukweli,
    );
    contract(
        &mut functions,
        "ni_faili",
        vec![ValueType::Neno],
        ValueType::Ukweli,
    );
    contract(
        &mut functions,
        "njia_unganisha",
        vec![ValueType::Neno, ValueType::Neno],
        ValueType::Neno,
    );
    contract(
        &mut functions,
        "njia_mzazi",
        vec![ValueType::Neno],
        ValueType::Chaguo(Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "njia_jina",
        vec![ValueType::Neno],
        ValueType::Chaguo(Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "njia_kiendelezi",
        vec![ValueType::Neno],
        ValueType::Chaguo(Box::new(ValueType::Neno)),
    );
    contract(
        &mut functions,
        "njia_kamili",
        vec![ValueType::Neno],
        ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
    );
    functions.insert(
        "soma_faili".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "andika_faili".to_string(),
        FnContract {
            params: vec![ValueType::Neno, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "ongeza".to_string(),
        FnContract {
            params: vec![ValueType::Neno, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "vipo".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Ukweli,
        },
    );
    functions.insert(
        "futa".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "ukubwa".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Namba,
        },
    );
    functions.insert(
        "faili_fungua".to_string(),
        FnContract {
            params: vec![ValueType::Neno, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Faili), Box::new(ValueType::Neno)),
        },
    );
    let mut constants = HashMap::new();
    constants.insert("NJIA_SEPARATOR".to_string(), ValueType::Neno);
    BuiltinExportTable {
        functions,
        constants,
    }
}

/// Sambamba (concurrency): tenda/subiri_tenda (thread spawn/join, 1:1 OS-thread model), njia
/// (channel), fungo (mutex). Requires `leta sambamba`. `tenda` is variadic (kazi name + however
/// many args that kazi takes) — special-cased by name in the analyzer's arity check, matching
/// the existing `orodha`/`seti` precedent, not a general FnContract flag.
pub fn sambamba_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "tenda".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "subiri_tenda".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "njia".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Jozi(
                Box::new(ValueType::NjiaTx(Box::new(ValueType::Unknown))),
                Box::new(ValueType::NjiaRx(Box::new(ValueType::Unknown))),
            ),
        },
    );
    functions.insert(
        "njia_na_kikomo".to_string(),
        FnContract {
            params: vec![ValueType::Namba],
            ret: ValueType::Jozi(
                Box::new(ValueType::NjiaTxBounded(Box::new(ValueType::Unknown))),
                Box::new(ValueType::NjiaRxBounded(Box::new(ValueType::Unknown))),
            ),
        },
    );
    functions.insert(
        "fungo".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::Tokeo(
                Box::new(ValueType::Fungo(Box::new(ValueType::Unknown))),
                Box::new(ValueType::Neno),
            ),
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Kiungo (FFI): load lib, call symbol. Requires `leta kiungo`. Stub returns error.
pub fn kiungo_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "saza_kiungo".to_string(),
        FnContract {
            params: vec![ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Anuani), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "wito_kiungo".to_string(),
        FnContract {
            params: vec![ValueType::Anuani, ValueType::Neno],
            ret: ValueType::Tokeo(Box::new(ValueType::Unknown), Box::new(ValueType::Neno)),
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Syscall: raw system call. Requires `leta syscall`. Stub returns 0.
pub fn syscall_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "syscall".to_string(),
        FnContract {
            params: vec![
                ValueType::Namba,
                ValueType::Namba,
                ValueType::Namba,
                ValueType::Namba,
            ],
            ret: ValueType::Anuani,
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Runtime (kitekelezi): version, platform. Requires `leta runtime`.
pub fn runtime_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "toleo".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Neno,
        },
    );
    functions.insert(
        "jina_os".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Neno,
        },
    );
    // These were already implemented (core/evaluator/src/builtins/runtime.rs) and callable at
    // runtime, but missing from this export table — so `leta runtime` alone was never enough to
    // actually use them: the semantic checker rejected every call with SEM037 "kazi haijulikani".
    functions.insert(
        "arch".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Neno,
        },
    );
    functions.insert(
        "ni_debug".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Ukweli,
        },
    );
    functions.insert(
        "ni_wasm".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Ukweli,
        },
    );
    functions.insert(
        "mazingira".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Kamusi(Box::new(ValueType::Neno), Box::new(ValueType::Neno)),
        },
    );
    functions.insert(
        "muda_wa_kuanza".to_string(),
        FnContract {
            params: vec![],
            ret: ValueType::Wakati,
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Kasha_GC (managed memory): reference-counted shared wrapper. Requires `leta kasha_gc`. Not in
/// the default prelude — opt-in, layered on top of the ownership model (see spec's roadmap).
pub fn kasha_gc_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    functions.insert(
        "kasha_gc_unda".to_string(),
        FnContract {
            params: vec![ValueType::Unknown],
            ret: ValueType::KashaGC(Box::new(ValueType::Unknown)),
        },
    );
    functions.insert(
        "kasha_gc_dhaifu".to_string(),
        FnContract {
            params: vec![ValueType::KashaGC(Box::new(ValueType::Unknown))],
            ret: ValueType::KashaGCDhaifu(Box::new(ValueType::Unknown)),
        },
    );
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

fn tokeo_neno(ok: ValueType) -> ValueType {
    ValueType::Tokeo(Box::new(ok), Box::new(ValueType::Neno))
}

/// Regular expressions: `ruwaza_*(ruwaza, maandishi, ...)`, each a `Tokeo` whose error is an
/// invalid pattern.
pub fn ruwaza_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    let texts = || ValueType::Orodha(Box::new(ValueType::Neno));
    let two = || vec![ValueType::Neno, ValueType::Neno];
    contract(
        &mut functions,
        "ruwaza_inalingana",
        two(),
        tokeo_neno(ValueType::Ukweli),
    );
    contract(
        &mut functions,
        "ruwaza_tafuta",
        two(),
        tokeo_neno(ValueType::Chaguo(Box::new(ValueType::Neno))),
    );
    contract(&mut functions, "ruwaza_zote", two(), tokeo_neno(texts()));
    contract(
        &mut functions,
        "ruwaza_vikundi",
        two(),
        tokeo_neno(ValueType::Chaguo(Box::new(texts()))),
    );
    contract(
        &mut functions,
        "ruwaza_badilisha",
        vec![ValueType::Neno, ValueType::Neno, ValueType::Neno],
        tokeo_neno(ValueType::Neno),
    );
    contract(&mut functions, "ruwaza_gawanya", two(), tokeo_neno(texts()));
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Encodings, hashes and identifiers.
pub fn usimbaji_exports() -> BuiltinExportTable {
    let mut functions = HashMap::new();
    for name in ["base64_simba", "hex_simba", "hashi_sha256", "hashi_sha512"] {
        contract(&mut functions, name, vec![ValueType::Neno], ValueType::Neno);
    }
    contract(
        &mut functions,
        "base64_fumbua",
        vec![ValueType::Neno],
        tokeo_neno(ValueType::Neno),
    );
    contract(
        &mut functions,
        "hmac_sha256",
        vec![ValueType::Neno, ValueType::Neno],
        ValueType::Neno,
    );
    contract(&mut functions, "kitambulisho", vec![], ValueType::Neno);
    BuiltinExportTable {
        functions,
        constants: HashMap::new(),
    }
}

/// Return builtin export table for the given module name, or None.
pub fn builtin_module_exports(name: &str) -> Option<BuiltinExportTable> {
    match name {
        "msingi" => Some(msingi_exports()),
        "mfumo" => Some(mfumo_exports()),
        "majira" => Some(majira_exports()),
        "matumizi" => Some(matumizi_exports()),
        "faili" => Some(faili_exports()),
        "hisabati" => Some(hisabati_exports()),
        "runtime" => Some(runtime_exports()),
        "syscall" => Some(syscall_exports()),
        "kiungo" => Some(kiungo_exports()),
        "sambamba" => Some(sambamba_exports()),
        "kasha_gc" => Some(kasha_gc_exports()),
        "ruwaza" => Some(ruwaza_exports()),
        "usimbaji" => Some(usimbaji_exports()),
        _ => None,
    }
}

/// Types with built-in methods (the receivers of `x.njia(...)` that are not a `umbo` or
/// `jenum`). The method tables below are indexed by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodReceiver {
    Neno,
    Orodha,
    Kamusi,
    Seti,
    Chaguo,
    Tokeo,
    Jozi,
    Wakati,
    KashaGC,
    KashaGCDhaifu,
    Faili,
    Mkondo,
    Kumbukumbu,
    NjiaTx,
    NjiaRx,
    Fungo,
}

/// Built-in methods that only read their receiver, per [`MethodReceiver`]. The one list: the
/// semantic analyzer accepts exactly these (with [`MUTATING_METHODS`] and
/// [`CALLBACK_METHODS`]), and both engines dispatch on them.
pub const PURE_METHODS: [&[&str]; 16] = [
    &[
        "clona",
        "urefu",
        "herufi_kwa",
        "biti_ngapi",
        "unganisha",
        "kata",
        "tafuta",
        "kwa_herufi_ndogo",
        "kwa_herufi_kubwa",
        "tupu",
        "ina",
        "hesabu",
        "rudia",
        "anza_na",
        "maliza_na",
        "gawanya",
        "badilisha",
        "safisha",
        "safisha_mwanzo",
        "safisha_mwisho",
        "jaza_kushoto",
        "jaza_kulia",
        "jaza",
        "herufi",
        "mistari",
        "geuza",
        "misimbo",
        "kwa_namba",
    ],
    &[
        "clona",
        "urefu",
        "pata",
        "unganisha",
        "jiunge",
        "kwa_neno",
        "vipande",
        "tupu",
        "kwanza",
        "mwisho",
        "ina",
        "tafuta",
        "kata",
        "geuza",
        "panga",
        "kubwa",
        "ndogo",
        "jumla",
        "kipekee",
    ],
    &[
        "clona",
        "idadi",
        "pata",
        "funguo",
        "vipo",
        "thamani",
        "vipengele",
    ],
    &[
        "ina",
        "urefu",
        "clona",
        "orodha",
        "muungano",
        "makutano",
        "tofauti",
        "ni_sehemu_ya",
    ],
    &["angu", "ni_tupu", "ni_po", "hakikisha"],
    &["ni_kosa", "ni_sawa", "kosa", "angu"],
    &["clona", "kwanza", "pili"],
    &["sekunde"],
    &["pata", "weka", "idadi", "shirikisha"],
    &["imarisha"],
    &["soma", "andika", "funga"],
    &["soma", "andika", "funga", "soma_bailisi"],
    &["pata"],
    &["tuma"],
    &["pokea"],
    &["funga", "fungua", "pata", "weka"],
];

/// Built-in methods that change their receiver in place, per [`MethodReceiver`].
pub const MUTATING_METHODS: [&[&str]; 16] = [
    &[],
    &[
        "ongeza",
        "ingiza",
        "ondoa",
        "badilisha",
        "ongeza_zote",
        "futa_zote",
    ],
    &["ingiza", "weka_key", "ondoa", "futa_zote"],
    &["ongeza", "ondoa"],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
];

/// Built-in methods that call a `kazi` (or builtin) named by their first argument, per element.
pub const CALLBACK_METHODS: [&[&str]; 16] = [
    &[],
    &[
        "ramani",
        "chuja",
        "hesabu",
        "chunguza",
        "kila_na_fahirisi",
        "kila_mmoja",
        "panga_kwa",
    ],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
];

/// Whether `receiver` has a built-in method `name`.
pub fn has_builtin_method(receiver: MethodReceiver, name: &str) -> bool {
    let i = receiver as usize;
    [&PURE_METHODS, &MUTATING_METHODS, &CALLBACK_METHODS]
        .iter()
        .any(|table| table[i].contains(&name))
}

/// Static type of the result of built-in method `method` on a value of type `receiver`
/// (`Unknown` when it has none or is not known statically). The one table of built-in method
/// result types: the analyzer checks calls with it, and the bytecode compiler follows method
/// chains with it.
pub fn method_return_type(receiver: &ValueType, method: &str) -> ValueType {
    match (receiver.clone(), method) {
        (ValueType::Neno, "clona") => ValueType::Neno,
        (ValueType::Neno, "urefu" | "biti_ngapi") => ValueType::Namba,
        (ValueType::Neno, "herufi_kwa") => ValueType::Chaguo(Box::new(ValueType::Herufi)),
        (ValueType::Neno, "kwa_herufi_ndogo" | "kwa_herufi_kubwa" | "badilisha" | "rudia") => {
            ValueType::Neno
        }
        (ValueType::Neno, "tupu" | "anza_na" | "maliza_na" | "ina") => ValueType::Ukweli,
        (ValueType::Neno, "hesabu") => ValueType::Namba,
        (ValueType::Neno, "gawanya") => ValueType::Orodha(Box::new(ValueType::Neno)),
        (
            ValueType::Neno,
            "kata" | "safisha" | "safisha_mwanzo" | "safisha_mwisho" | "jaza_kushoto"
            | "jaza_kulia" | "jaza" | "geuza",
        ) => ValueType::Neno,
        (ValueType::Neno, "herufi" | "mistari") => ValueType::Orodha(Box::new(ValueType::Neno)),
        (ValueType::Neno, "misimbo") => ValueType::Orodha(Box::new(ValueType::Namba)),
        (ValueType::Neno, "kwa_namba") => {
            ValueType::Tokeo(Box::new(ValueType::Namba), Box::new(ValueType::Neno))
        }
        (ValueType::Neno, "tafuta") => ValueType::Chaguo(Box::new(ValueType::Namba)),
        (ValueType::Jozi(k, v), "clona") => ValueType::Jozi(k.clone(), v.clone()),
        (ValueType::Jozi(k, _), "kwanza") => *k,
        (ValueType::Jozi(_, v), "pili") => *v,
        (ValueType::Orodha(ref t), "clona") => ValueType::Orodha(t.clone()),
        (ValueType::Orodha(_), "urefu") => ValueType::Namba,
        (ValueType::Orodha(_), "ongeza" | "ongeza_zote" | "futa_zote") => ValueType::Tupu,
        (ValueType::Orodha(_), "tupu" | "ina") => ValueType::Ukweli,
        (ValueType::Orodha(ref t), "kwanza" | "mwisho" | "kubwa" | "ndogo") => {
            ValueType::Chaguo(t.clone())
        }
        (ValueType::Orodha(_), "tafuta") => ValueType::Chaguo(Box::new(ValueType::Namba)),
        (ValueType::Orodha(ref t), "kata" | "geuza" | "panga" | "panga_kwa" | "kipekee") => {
            ValueType::Orodha(t.clone())
        }
        (ValueType::Orodha(_), "jumla") => ValueType::Namba,
        (ValueType::Orodha(_), "ingiza") => ValueType::Tupu,
        (ValueType::Orodha(ref t), "ondoa") => ValueType::Chaguo(t.clone()),
        (ValueType::Orodha(ref t), "pata") => ValueType::Chaguo(t.clone()),
        (ValueType::Orodha(_), "badilisha" | "kila_mmoja" | "kila_na_fahirisi") => ValueType::Tupu,
        (ValueType::Orodha(ref t), "ramani" | "chuja") => ValueType::Orodha(t.clone()),
        (ValueType::Orodha(_), "hesabu") => ValueType::Namba,
        (ValueType::Orodha(_), "chunguza") => ValueType::Ukweli,
        (ValueType::Orodha(_), "unganisha" | "jiunge") => ValueType::Neno,
        (ValueType::Orodha(_), "kwa_neno") => ValueType::Orodha(Box::new(ValueType::Neno)),
        (ValueType::Orodha(ref t), "vipande") => {
            ValueType::Orodha(Box::new(ValueType::Orodha(t.clone())))
        }
        (ValueType::Kamusi(ref k, ref v), "clona") => ValueType::Kamusi(k.clone(), v.clone()),
        (ValueType::Kamusi(_, _), "idadi") => ValueType::Namba,
        (ValueType::Kamusi(_, ref v), "pata") => ValueType::Chaguo(v.clone()),
        (ValueType::Kamusi(_, _), "ingiza" | "weka_key" | "futa_zote") => ValueType::Tupu,
        (ValueType::Kamusi(_, ref v), "ondoa") => ValueType::Chaguo(v.clone()),
        (ValueType::Kamusi(_, ref v), "thamani") => ValueType::Orodha(v.clone()),
        (ValueType::Kamusi(ref k, ref v), "vipengele") => {
            ValueType::Orodha(Box::new(ValueType::Jozi(k.clone(), v.clone())))
        }
        (ValueType::Kamusi(_, _), "vipo") => ValueType::Ukweli,
        (ValueType::Kamusi(ref k, _), "funguo") => ValueType::Orodha(k.clone()),
        (ValueType::Tokeo(ref t, _), "angu") => *t.clone(),
        (ValueType::Tokeo(_, _), "ni_kosa" | "ni_sawa") => ValueType::Ukweli,
        (ValueType::Tokeo(_, ref e), "kosa") => *e.clone(),
        (ValueType::Chaguo(ref t), "angu" | "hakikisha") => *t.clone(),
        (ValueType::Chaguo(_), "ni_po" | "ni_tupu") => ValueType::Ukweli,
        (ValueType::KashaGC(ref t), "pata") => *t.clone(),
        (ValueType::KashaGC(_), "weka") => ValueType::Tupu,
        (ValueType::KashaGC(_), "idadi") => ValueType::Namba,
        (ValueType::KashaGC(ref t), "shirikisha") => ValueType::KashaGC(t.clone()),
        (ValueType::KashaGCDhaifu(ref t), "imarisha") => {
            ValueType::Chaguo(Box::new(ValueType::KashaGC(t.clone())))
        }
        (ValueType::Faili, "soma") => {
            ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno))
        }
        (ValueType::Faili, "andika") => {
            ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno))
        }
        (ValueType::Faili, "funga") => ValueType::Tupu,
        (ValueType::Mkondo, "soma") => {
            ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno))
        }
        (ValueType::Mkondo, "soma_bailisi") => {
            ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno))
        }
        (ValueType::Mkondo, "andika") => {
            ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno))
        }
        (ValueType::Mkondo, "funga") => ValueType::Tupu,
        (ValueType::Kumbukumbu(ref t), "pata") => *t.clone(),
        (ValueType::Seti(ref t), "clona") => ValueType::Seti(t.clone()),
        (ValueType::Seti(_), "ongeza") => ValueType::Tupu,
        (ValueType::Seti(_), "ondoa") => ValueType::Ukweli,
        (ValueType::Seti(_), "ina" | "ni_sehemu_ya") => ValueType::Ukweli,
        (ValueType::Seti(ref t), "muungano" | "makutano" | "tofauti") => ValueType::Seti(t.clone()),
        (ValueType::Seti(_), "urefu") => ValueType::Namba,
        (ValueType::Seti(ref t), "orodha") => ValueType::Orodha(t.clone()),
        (ValueType::NjiaTx(_), "tuma") => {
            ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno))
        }
        (ValueType::NjiaRx(ref t), "pokea") => {
            ValueType::Tokeo(t.clone(), Box::new(ValueType::Neno))
        }
        (ValueType::NjiaTxBounded(_), "tuma") => {
            ValueType::Tokeo(Box::new(ValueType::Tupu), Box::new(ValueType::Neno))
        }
        (ValueType::NjiaRxBounded(ref t), "pokea") => {
            ValueType::Tokeo(t.clone(), Box::new(ValueType::Neno))
        }
        (ValueType::Wakati, "sekunde") => ValueType::Namba,
        (ValueType::Fungo(_), "funga") => ValueType::Tupu,
        (ValueType::Fungo(_), "fungua") => ValueType::Tupu,
        (ValueType::Fungo(ref t), "pata") => *t.clone(),
        (ValueType::Fungo(_), "weka") => ValueType::Tupu,
        _ => ValueType::Unknown,
    }
}
