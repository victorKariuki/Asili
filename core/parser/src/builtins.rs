//! Builtin module export tables (msingi, mfumo, majira, matumizi, faili, hisabati, ruwaza,
//! usimbaji, ...). Resolved without disk I/O.

use crate::{FnContract, ValueType};
use std::collections::HashMap;
use std::sync::OnceLock;

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

/// The language's built-in type names, as written in source — the one list editors (LSP
/// completion, the VS Code grammar, the playground) offer and highlight. Builtin `umbo`s
/// ([`builtin_structs`]) come on top.
pub const BUILTIN_TYPE_NAMES: &[&str] = &[
    "Namba",
    "Neno",
    "Baiti",
    "Ukweli",
    "Herufi",
    "Tupu",
    "Hamna",
    "Orodha",
    "Kamusi",
    "Seti",
    "Jozi",
    "Chaguo",
    "Tokeo",
    "Wakati",
    "Anuani",
    "Namba_Kuu",
    "Namba_Sahihi",
    "Kumbukumbu",
    "Kasha_GC",
    "Kasha_GC_Dhaifu",
    "Faili",
    "Mkondo",
    "MkondoSikilizaji",
    "TlsUsanidi",
    "NjiaTx",
    "NjiaRx",
    "Fungo",
    "Ahadi",
    "Biti8",
    "Biti16",
    "Biti32",
    "Biti64",
    "uBiti8",
    "uBiti16",
    "uBiti32",
    "uBiti64",
];

/// Export table: functions and constants for a builtin module.
#[derive(Clone, Debug, Default)]
pub struct BuiltinExportTable {
    pub functions: HashMap<String, FnContract>,
    pub constants: HashMap<String, ValueType>,
}

/// One builtin module's surface, written in Asili: the single source for the analyzer's
/// contracts, LSP hover and signature help, and the generated `lib/std/<name>.asi` stubs.
///
/// A function is `(signature, doc)` with the signature `name(p: T, q?: T, ...r: T) -> R`: `q?`
/// may be left out of a call (trailing parameters only), `...r` takes any number of arguments of
/// type `T`. A constant is `(NAME: T, doc)`. A struct ([`StructSrc`]) is a `umbo` a builtin
/// takes or returns, usable without declaring it.
pub struct BuiltinModule {
    pub name: &'static str,
    pub doc: &'static str,
    pub functions: &'static [(&'static str, &'static str)],
    pub constants: &'static [(&'static str, &'static str)],
    pub structs: &'static [StructSrc],
}

/// A builtin `umbo` as [`BUILTIN_MODULES`] writes it: each field `(name: T, doc)`, where a `?`
/// after the name lets a literal leave the field out (it then holds `Hamna`).
pub struct StructSrc {
    pub name: &'static str,
    pub doc: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}

pub const BUILTIN_MODULES: &[BuiltinModule] = &[
    BuiltinModule {
        name: "msingi",
        doc: "Msingi: aina na viunda vya msingi; viko wigoni bila `leta`.",
        functions: &[
            ("baiti(vipengele: Orodha<Namba>) -> Tokeo<Baiti, Neno>", "Baiti kutoka namba kamili 0–255; kosa kipengele kisipokuwa hivyo."),
            ("baiti_ya_nambari(namba: Namba, upana: Namba, mpangilio: Neno) -> Tokeo<Baiti, Neno>", "Namba kamili isiyo hasi kama baiti `upana` (1, 2, 4 au 8), mpangilio \"be\" (kubwa kwanza) au \"le\"."),
            ("chaguo(thamani: Haijulikani) -> Haijulikani?", "`Chaguo::Kuna(thamani)`."),
            ("jozi(a: Haijulikani, b: Haijulikani) -> Jozi<Haijulikani, Haijulikani>", "Jozi ya thamani mbili."),
            ("kamusi() -> Kamusi<Haijulikani, Haijulikani>", "Kamusi tupu."),
            ("kamusi_tupu() -> Kamusi<Haijulikani, Haijulikani>", "Kamusi tupu."),
            ("kosa(ujumbe: Neno) -> Tokeo<Haijulikani, Neno>", "`Tokeo::Kosa(ujumbe)`."),
            ("kumbukumbu_unda(thamani: Haijulikani) -> Kumbukumbu<Haijulikani>", "Kisanduku cha heap kinachomiliki thamani."),
            ("orodha(...vipengele: Haijulikani) -> Orodha<Haijulikani>", "Orodha ya vipengele vilivyotolewa."),
            ("orodha_rudia(thamani: Haijulikani, idadi: Namba) -> Orodha<Haijulikani>", "Orodha ya `thamani` mara `idadi`."),
            ("seti(...vipengele: Haijulikani) -> Seti<Haijulikani>", "Seti ya vipengele vilivyotolewa (marudio huondolewa)."),
            ("seti_tupu() -> Seti<Haijulikani>", "Seti tupu."),
            ("tokeo(thamani: Haijulikani) -> Tokeo<Haijulikani, Haijulikani>", "`Tokeo::Sawa(thamani)`."),
        ],
        constants: &[
            ("KWELI: Ukweli", ""),
            ("SIYO_KWELI: Ukweli", ""),
            ("TUPU: Tupu", ""),
        ],
        structs: &[],
    },
    BuiltinModule {
        name: "hisabati",
        doc: "Hisabati: hesabu, trigonometria, logi na namba za nasibu.",
        functions: &[
            ("abs(n: Namba) -> Namba", "Thamani kamili (bila ishara)."),
            ("absolute(n: Namba) -> Namba", "Thamani kamili (bila ishara)."),
            ("akosini(x: Namba) -> Tokeo<Namba, Neno>", "Arccos, kwa radiani; kosa nje ya [-1, 1]."),
            ("akosini_h(x: Namba) -> Tokeo<Namba, Neno>", "Arccosh; kosa chini ya 1."),
            ("asini(x: Namba) -> Tokeo<Namba, Neno>", "Arcsin, kwa radiani; kosa nje ya [-1, 1]."),
            ("asini_h(x: Namba) -> Tokeo<Namba, Neno>", "Arcsinh."),
            ("atanjenti(x: Namba) -> Tokeo<Namba, Neno>", "Arctan, kwa radiani."),
            ("atanjenti2(y: Namba, x: Namba) -> Namba", "Pembe ya nukta (x, y) kutoka mhimili wa x, kwa radiani."),
            ("atanjenti_h(x: Namba) -> Tokeo<Namba, Neno>", "Arctanh; kosa nje ya (-1, 1)."),
            ("baki(x: Namba, y: Namba) -> Tokeo<Namba, Neno>", "Baki la x / y; kosa y ikiwa 0."),
            ("chagua_nasibu(orodha: Orodha<Haijulikani>) -> Haijulikani?", "Kipengele kimoja cha nasibu, au `Hamna` orodha ikiwa tupu."),
            ("changanya(orodha: Orodha<Haijulikani>) -> Orodha<Haijulikani>", "Nakala ya orodha kwa mpangilio wa nasibu."),
            ("dari(n: Namba) -> Namba", "Namba kamili ndogo zaidi isiyo chini ya n."),
            ("duara(n: Namba) -> Namba", "n kwa namba kamili iliyo karibu zaidi."),
            ("duara_maeneo(x: Namba, m: Namba) -> Namba", "x kwa maeneo m ya desimali."),
            ("expm1(x: Namba) -> Namba", "e^x - 1, sahihi kwa x ndogo."),
            ("faktoriali(n: Namba) -> Tokeo<Namba, Neno>", "n!; kosa kwa n hasi au isiyo kamili."),
            ("gawio(a: Namba, b: Namba) -> Tokeo<Namba, Neno>", "a / b; kosa b ikiwa 0."),
            ("haipot(x: Namba, y: Namba) -> Namba", "Urefu wa hipotenusi: √(x² + y²)."),
            ("ishara(n: Namba) -> Namba", "-1, 0 au 1 kulingana na ishara ya n."),
            ("jumla(a: Namba, b: Namba) -> Namba", "a + b."),
            ("kikwazo(x: Namba, chini: Namba, juu: Namba) -> Namba", "x ikibanwa kati ya chini na juu."),
            ("kipeo(msingi: Namba, nguvu: Namba) -> Tokeo<Namba, Neno>", "msingi kwa nguvu; kosa matokeo yasipokuwa namba halisi."),
            ("kipeuo2(n: Namba) -> Tokeo<Namba, Neno>", "Kipeuo cha pili; kosa kwa n hasi."),
            ("kipeuo3(n: Namba) -> Namba", "Kipeuo cha tatu."),
            ("kosini(x: Namba) -> Namba", "Cos ya x (radiani)."),
            ("kosini_h(x: Namba) -> Namba", "Cosh ya x."),
            ("kubwa(a: Namba, b: Namba) -> Namba", "Kubwa kati ya a na b."),
            ("kwenda_nyuzi(r: Namba) -> Namba", "Radiani kwenda nyuzi (digrii)."),
            ("kwenda_radiani(d: Namba) -> Namba", "Nyuzi (digrii) kwenda radiani."),
            ("logi(x: Namba) -> Tokeo<Namba, Neno>", "Logi asilia; kosa kwa x isiyo chanya."),
            ("logi10(x: Namba) -> Tokeo<Namba, Neno>", "Logi ya msingi 10; kosa kwa x isiyo chanya."),
            ("logi1p(x: Namba) -> Tokeo<Namba, Neno>", "Logi asilia ya 1 + x, sahihi kwa x ndogo."),
            ("logi2(x: Namba) -> Tokeo<Namba, Neno>", "Logi ya msingi 2; kosa kwa x isiyo chanya."),
            ("mizizi(n: Namba) -> Tokeo<Namba, Neno>", "Kipeuo cha pili; kosa kwa n hasi."),
            ("mzizi(x: Namba, n: Namba) -> Tokeo<Namba, Neno>", "Kipeuo cha n cha x."),
            ("namba_kuu_kutoka(maandishi: Neno) -> Tokeo<Namba_Kuu, Neno>", "Namba kamili kubwa bila kikomo kutoka maandishi."),
            ("namba_sahihi_kutoka(maandishi: Neno) -> Tokeo<Namba_Sahihi, Neno>", "Desimali sahihi kutoka maandishi."),
            ("nasibu() -> Namba", "Namba ya nasibu katika [0, 1)."),
            ("nasibu_chini(chini: Namba, juu: Namba) -> Namba", "Namba ya nasibu katika [chini, juu)."),
            ("nasibu_kamili(chini: Namba, juu: Namba) -> Namba", "Namba kamili kati ya chini na juu (zote mbili zimo)."),
            ("nasibu_mbegu(mbegu: Namba) -> Tupu", "Huanza mfululizo unaorudiwa wa namba za nasibu (kwa majaribio)."),
            ("ndogo(a: Namba, b: Namba) -> Namba", "Ndogo kati ya a na b."),
            ("ni_namba(x: Namba) -> Ukweli", "Kweli x isipokuwa NaN."),
            ("ni_ukomo(x: Namba) -> Ukweli", "Kweli x ikiwa ukomo (chanya au hasi)."),
            ("punguza(n: Namba) -> Namba", "n bila sehemu ya desimali (kuelekea 0)."),
            ("sakafu(n: Namba) -> Namba", "Namba kamili kubwa zaidi isiyozidi n."),
            ("si_namba(x: Namba) -> Ukweli", "Kweli x ikiwa NaN."),
            ("sini(x: Namba) -> Namba", "Sin ya x (radiani)."),
            ("sini_h(x: Namba) -> Namba", "Sinh ya x."),
            ("tanjenti(x: Namba) -> Namba", "Tan ya x (radiani)."),
            ("tanjenti_h(x: Namba) -> Namba", "Tanh ya x."),
            ("tofauti(a: Namba, b: Namba) -> Namba", "a - b."),
            ("upeo(msingi: Namba, nguvu: Namba) -> Tokeo<Namba, Neno>", "msingi kwa nguvu; kosa matokeo yasipokuwa namba halisi."),
            ("upeo_wa_e(x: Namba) -> Namba", "e kwa nguvu x."),
            ("zao(a: Namba, b: Namba) -> Namba", "a × b."),
        ],
        constants: &[
            ("E: Namba", "Namba ya Euler."),
            ("EPSILON: Namba", "Tofauti ndogo zaidi kati ya 1 na Namba inayofuata."),
            ("INF: Namba", "Ukomo chanya."),
            ("KIPEUO1_2: Namba", "1 / √2."),
            ("KIPEUO2: Namba", "√2."),
            ("KIPEUO3: Namba", "√3."),
            ("KIPEUO5: Namba", "√5."),
            ("LN10: Namba", "Logi asilia ya 10."),
            ("LN2: Namba", "Logi asilia ya 2."),
            ("LOG10E: Namba", "Logi ya msingi 10 ya e."),
            ("LOG2E: Namba", "Logi ya msingi 2 ya e."),
            ("NAN: Namba", "Si namba (NaN)."),
            ("PHI: Namba", "Uwiano wa dhahabu."),
            ("PI: Namba", "π."),
            ("Siyo_Namba: Namba", "Si namba (NaN)."),
            ("TAU: Namba", "2π."),
            ("Ukomo: Namba", "Ukomo chanya."),
        ],
        structs: &[],
    },
    BuiltinModule {
        name: "mfumo",
        doc: "Mfumo: mazingira, programu nyingine, JSON, mtandao (TCP, TLS, HTTP) na ishara.",
        functions: &[
            ("endesha(amri: Neno, hoja: Orodha<Neno>) -> Tokeo<Neno, Neno>", "Endesha programu nyingine; matokeo yake (stdout) ikifaulu, au kosa lenye msimbo na stderr."),
            ("http_ombi(njia: Neno, anwani: Neno, chaguo?: ChaguoHttp) -> Tokeo<JibuHttp, Neno>", "Ombi la HTTP(S) la njia yoyote (GET, POST, ...). Jibu zima kwa hali yoyote; kosa ni la muunganisho tu (au la hali isiyo 2xx, `kosa_hali` ikiwa kweli)."),
            ("kikomo_kumbukumbu(baiti: Namba) -> Tupu", "Kikomo cha kumbukumbu ya programu, kwa baiti (0: hakuna); kukipita husimamisha programu."),
            ("kutoka_json(maandishi: Neno) -> Tokeo<Kamusi<Neno, Haijulikani>, Neno>", "Thamani kutoka maandishi ya JSON."),
            ("kwa_json(thamani: Haijulikani) -> Tokeo<Neno, Neno>", "Maandishi ya JSON ya thamani."),
            ("mkondo_sikiliza(anwani: Neno) -> Tokeo<MkondoSikilizaji, Neno>", "Sikiliza miunganisho ya TCP kwenye anwani (\"mwenyeji:mlango\")."),
            ("mkondo_tumikia(sikilizaji: MkondoSikilizaji, kazi_jina: Neno, idadi_ya_nyuzi: Namba, tls?: TlsUsanidi?) -> Tokeo<Tupu, Neno>", "Hudumia kila muunganisho kwa kazi `kazi_jina(Mkondo)` kwenye nyuzi `idadi_ya_nyuzi`."),
            ("mkondo_tumikia_http(sikilizaji: MkondoSikilizaji, kazi_jina: Neno, idadi_ya_nyuzi: Namba, tls?: TlsUsanidi?) -> Tokeo<Tupu, Neno>", "Seva ya HTTP/1.1: kila ombi ni `kazi_jina(OmbiHttp) -> JibuHttp`."),
            ("mkondo_unganisha(anwani: Neno) -> Tokeo<Mkondo, Neno>", "Unganisha kwa TCP kwenye anwani (\"mwenyeji:mlango\")."),
            ("mlinzi_anza(ms: Namba) -> Tokeo<Tupu, Neno>", "Anza mlinzi: programu isipomlisha ndani ya ms milisekunde, husimamishwa."),
            ("mlinzi_lisha() -> Tupu", "Lisha mlinzi aliyeanzishwa na `mlinzi_anza`."),
            ("pata_env(jina: Neno) -> Neno?", "Thamani ya kigezo cha mazingira, au `Hamna`."),
            ("rejesha_ishara(ishara: Namba) -> Tokeo<Tupu, Neno>", "Rudisha ishara ya OS kwenye ushughulikiaji wake wa kawaida."),
            ("sikiliza_ishara(ishara: Namba, kazi_jina: Neno) -> Tokeo<Tupu, Neno>", "Ita kazi `kazi_jina` ishara ya OS ikifika (Unix)."),
            ("tls_sanidi(cheti_njia: Neno, ufunguo_njia: Neno) -> Tokeo<TlsUsanidi, Neno>", "Pakia cheti na ufunguo (PEM) kwa seva ya TLS."),
            ("toka(kodi: Namba) -> Tupu", "Maliza programu kwa msimbo wa kutoka."),
            ("vigezo() -> Orodha<Neno>", "Hoja za mstari wa amri."),
            ("weka_env(jina: Neno, thamani: Neno) -> Tupu", "Weka kigezo cha mazingira."),
        ],
        constants: &[
            ("JINA_OS: Neno", "Jina la mfumo wa uendeshaji."),
            ("TOLEO: Neno", "Toleo la Asili."),
        ],
        structs: &[
            StructSrc {
                name: "OmbiHttp",
                doc: "Ombi ambalo seva ya `mkondo_tumikia_http` hupitisha kwa kazi yake.",
                fields: &[
                    ("njia: Neno", "GET, POST, ..."),
                    ("anwani: Neno", "Njia na hoja za ombi (\"/bidhaa?id=7\")."),
                    ("vichwa: Kamusi<Neno, Neno>", "Vichwa vya ombi, majina kwa herufi ndogo."),
                    ("mwili: Neno", "Mwili wa ombi."),
                ],
            },
            StructSrc {
                name: "JibuHttp",
                doc: "Jibu la HTTP: kazi ya seva hulirudisha, `http_ombi` hulipokea.",
                fields: &[
                    ("hali: Namba", "Msimbo wa hali (200, 404, ...)."),
                    ("vichwa: Kamusi<Neno, Neno>", "Vichwa, majina kwa herufi ndogo; kichwa kinachorudiwa huunganishwa kwa \", \"."),
                    ("mwili: Neno", "Mwili (tupu kwa `hifadhi`; base64 kwa `jibu_base64`)."),
                    ("sababu?: Neno", "Maelezo ya hali (\"Not Found\")."),
                    ("anwani?: Neno", "Anwani ya mwisho, baada ya kuelekezwa."),
                    ("toleo?: Neno", "Toleo la HTTP (\"HTTP/1.1\")."),
                    ("vichwa_vyote?: Orodha<Jozi<Neno, Neno>>", "Kila kichwa kwa mpangilio, marudio yakiwa tofauti (`set-cookie`); seva huvituma vyote."),
                    ("muda?: Namba", "Sekunde ombi lilizochukua, pamoja na kuelekezwa na kujaribu tena."),
                ],
            },
            StructSrc {
                name: "ChaguoHttp",
                doc: "Chaguo za `http_ombi`; kila uga ni wa hiari. Mwili ni mmoja tu kati ya `mwili`, `mwili_base64`, `json`, `fomu`, `fomu_sehemu`/`fomu_faili` na `faili`.",
                fields: &[
                    ("vichwa?: Kamusi<Neno, Neno>", "Vichwa vya ombi (\"User-Agent\", \"Accept\", ...)."),
                    ("hoja?: Kamusi<Neno, Neno>", "Huongezwa kwenye anwani kama `?jina=thamani`, zikisimbwa."),
                    ("mwili?: Neno", "Mwili wa maandishi (text/plain; charset=utf-8)."),
                    ("mwili_base64?: Neno", "Mwili wa baiti, ulioandikwa kwa base64 (application/octet-stream)."),
                    ("json?: Haijulikani", "Thamani yoyote, hutumwa kama JSON (application/json)."),
                    ("fomu?: Kamusi<Neno, Neno>", "Fomu ya application/x-www-form-urlencoded."),
                    ("fomu_sehemu?: Kamusi<Neno, Neno>", "Sehemu za maandishi za fomu ya multipart/form-data."),
                    ("fomu_faili?: Kamusi<Neno, Neno>", "Sehemu za faili za fomu ya multipart: jina la sehemu → njia ya faili."),
                    ("faili?: Neno", "Tuma faili hili kama mwili, bila kulisoma lote kwenye kumbukumbu."),
                    ("aina?: Neno", "Content-Type ya mwili, badala ya ile ya kawaida."),
                    ("mtumiaji?: Neno", "Uthibitisho wa Basic (pamoja na `nenosiri`)."),
                    ("nenosiri?: Neno", "Nenosiri la uthibitisho wa Basic."),
                    ("tokeni?: Neno", "Uthibitisho wa Bearer."),
                    ("muda?: Namba", "Sekunde za ombi lote (kawaida 60; 0: bila kikomo)."),
                    ("muda_kuunganisha?: Namba", "Sekunde za kuunganisha tu."),
                    ("elekezo?: Namba", "Idadi ya juu ya kuelekezwa kufuatwa (kawaida 10; 0: jibu la 3xx hurudishwa)."),
                    ("jaribu_tena?: Namba", "Majaribio zaidi (hadi 10) ya njia zisizobadilisha kitu, muunganisho ukishindwa au hali ikiwa 429/502/503/504; husubiri `Retry-After`."),
                    ("wakala?: Neno", "Wakala: \"http://mwenyeji:mlango\", \"socks5://...\" (mtumiaji:nenosiri@ yanaruhusiwa); \"\" huzima HTTPS_PROXY/HTTP_PROXY."),
                    ("familia_ip?: Namba", "4 au 6: tumia IPv4 au IPv6 pekee."),
                    ("cheti_ca?: Neno", "Faili la PEM la vyeti vya mamlaka vitakavyoaminiwa pekee (badala ya orodha ya Mozilla)."),
                    ("cheti?: Neno", "Cheti cha mteja (PEM) kwa TLS ya pande mbili, pamoja na `ufunguo`."),
                    ("ufunguo?: Neno", "Ufunguo wa siri (PEM) wa `cheti`."),
                    ("vidakuzi?: Ukweli", "Tumia mkebe mmoja wa vidakuzi wa programu nzima: hupokea `Set-Cookie`, hutuma `Cookie`."),
                    ("kikomo?: Namba", "Baiti za juu za mwili wa jibu (kawaida 64 MiB)."),
                    ("hifadhi?: Neno", "Andika mwili kwenye faili hili (kwa vipande; faili haliandikwi ombi likishindwa)."),
                    ("jibu_base64?: Ukweli", "Rudisha mwili kama base64 (kwa baiti zisizo maandishi)."),
                    ("kosa_hali?: Ukweli", "Hali isiyo 2xx iwe kosa lenye hali na mwanzo wa mwili."),
                ],
            },
        ],
    },
    BuiltinModule {
        name: "majira",
        doc: "Majira: saa, tarehe na muda (UTC, kalenda ya Gregori).",
        functions: &[
            ("kipima_muda() -> Namba", "Sekunde za saa isiyorudi nyuma, za kupima muda."),
            ("kutoka_iso(maandishi: Neno) -> Tokeo<Wakati, Neno>", "Wakati kutoka ISO 8601 (\"2026-10-09T12:30:00Z\", au yenye +03:00)."),
            ("kutoka_sekunde(n: Namba) -> Wakati", "Wakati wa sekunde n tangu 1970-01-01 UTC."),
            ("kutoka_tarehe(mwaka: Namba, mwezi: Namba, siku: Namba) -> Tokeo<Wakati, Neno>", "Wakati wa saa sita usiku (UTC) wa tarehe hiyo; kosa tarehe isipokuwepo."),
            ("kwa_iso(w: Wakati) -> Neno", "Tarehe na saa kwa ISO 8601 (UTC): \"2026-10-09T12:30:00Z\"."),
            ("lala(sekunde: Namba) -> Tupu", "Subiri kwa sekunde hizo."),
            ("majira() -> Namba", "Sekunde tangu 1970-01-01 UTC, sasa hivi."),
            ("sasa() -> Wakati", "Wakati wa sasa."),
            ("sekunde(w: Wakati) -> Namba", "Sekunde za w tangu 1970-01-01 UTC."),
            ("tarehe(w: Wakati) -> Kamusi<Neno, Namba>", "mwaka, mwezi, siku, saa, dakika, sekunde, siku_ya_wiki (1 = Jumatatu), siku_ya_mwaka."),
            ("umbiza(w: Wakati) -> Neno", "\"2026-10-09 12:30:00\" (UTC)."),
            ("umbiza_eneo(w: Wakati, dakika: Namba) -> Neno", "Kama umbiza, kwa saa za eneo lililo dakika `dakika` mbele ya UTC."),
        ],
        constants: &[
            ("MWANZO_WA_ZAMANI: Namba", "Sekunde za 1970-01-01 UTC (0)."),
            ("SEKUNDE_KWA_SIKU: Namba", "86400."),
        ],
        structs: &[],
    },
    BuiltinModule {
        name: "matumizi",
        doc: "Matumizi: kuandika na kusoma kwenye skrini.",
        functions: &[
            ("chapisha(ujumbe: Neno) -> Tupu", "Andika mstari kwenye stdout."),
            ("makosa(ujumbe: Neno) -> Tupu", "Andika \"KOSA: ujumbe\" kwenye stderr."),
            ("omba(swali: Neno) -> Neno", "Soma mstari mmoja kutoka stdin."),
            ("onyo(ujumbe: Neno) -> Tupu", "Andika onyo kwenye stderr."),
            ("paparika(ujumbe: Neno) -> Tupu", "Simamisha programu kwa ujumbe huo."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "faili",
        doc: "Faili: kusoma na kuandika faili, saraka na njia.",
        functions: &[
            ("andika_baiti(njia: Neno, data: Baiti) -> Tokeo<Tupu, Neno>", "Andika baiti kwenye faili (huunda au hufuta yaliyokuwepo)."),
            ("andika_faili(njia: Neno, data: Neno) -> Tokeo<Tupu, Neno>", "Andika data kwenye faili (huunda au hufuta yaliyokuwepo)."),
            ("badili_jina(kutoka: Neno, kwenda: Neno) -> Tokeo<Tupu, Neno>", "Hamisha au badili jina la faili au saraka."),
            ("faili_fungua(njia: Neno, hali: Neno) -> Tokeo<Faili, Neno>", "Kishikizo cha faili; hali ni \"soma\", \"andika\" au \"ongeza\"."),
            ("futa(njia: Neno) -> Tokeo<Tupu, Neno>", "Futa faili."),
            ("futa_saraka(njia: Neno) -> Tokeo<Tupu, Neno>", "Futa saraka pamoja na yaliyomo."),
            ("nakili(kutoka: Neno, kwenda: Neno) -> Tokeo<Namba, Neno>", "Nakili faili; idadi ya baiti zilizonakiliwa."),
            ("ni_faili(njia: Neno) -> Ukweli", "Kweli njia ikiwa faili."),
            ("ni_saraka(njia: Neno) -> Ukweli", "Kweli njia ikiwa saraka."),
            ("njia_jina(njia: Neno) -> Neno?", "Sehemu ya mwisho ya njia."),
            ("njia_kamili(njia: Neno) -> Tokeo<Neno, Neno>", "Njia kamili, bila `.`, `..` wala viungo."),
            ("njia_kiendelezi(njia: Neno) -> Neno?", "Kiendelezi cha jina (bila nukta)."),
            ("njia_mzazi(njia: Neno) -> Neno?", "Saraka inayoshikilia njia."),
            ("njia_unganisha(a: Neno, b: Neno) -> Neno", "Unganisha njia mbili (maandishi tu)."),
            ("ongeza(njia: Neno, data: Neno) -> Tokeo<Tupu, Neno>", "Ongeza data mwishoni mwa faili."),
            ("ongeza_baiti(njia: Neno, data: Baiti) -> Tokeo<Tupu, Neno>", "Ongeza baiti mwishoni mwa faili."),
            ("orodha_saraka(njia: Neno) -> Tokeo<Orodha<Neno>, Neno>", "Majina yaliyomo kwenye saraka, kwa mpangilio."),
            ("soma_baiti(njia: Neno) -> Tokeo<Baiti, Neno>", "Yaliyomo yote ya faili, kama baiti."),
            ("soma_faili(njia: Neno) -> Tokeo<Neno, Neno>", "Yaliyomo yote ya faili."),
            ("ukubwa(njia: Neno) -> Namba", "Ukubwa wa faili kwa baiti (0 lisipokuwepo)."),
            ("unda_saraka(njia: Neno) -> Tokeo<Tupu, Neno>", "Unda saraka pamoja na wazazi wake."),
            ("vipo(njia: Neno) -> Ukweli", "Kweli njia ikiwepo."),
        ],
        constants: &[("NJIA_SEPARATOR: Neno", "Kitenganishi cha njia cha mfumo (\"/\" au \"\\\\\").")],
        structs: &[],
    },
    BuiltinModule {
        name: "sambamba",
        doc: "Sambamba: nyuzi za OS (1:1), kazi za sawia (Ahadi), njia za ujumbe na kufuli. Kazi ya sawia ikisubiri (subiri, lala, njia, mtandao), kazi nyingine za uzi huo huendelea.",
        functions: &[
            ("anzisha(kazi_jina: Neno, ...hoja: Haijulikani) -> Ahadi<Haijulikani>", "Anzisha kazi `kazi_jina(hoja...)` kama kazi ya sawia; Ahadi yake (kama kuita `sawia kazi`)."),
            ("ghairi(ahadi: Ahadi<Haijulikani>) -> Tupu", "Ghairi kazi: kusubiri kwake kwa sasa (au kunakofuata) hurudisha kosa."),
            ("muda_kikomo(ahadi: Ahadi<Haijulikani>, sekunde: Namba) -> Haijulikani?", "Subiri kazi kwa sekunde hizo zaidi; `Hamna` muda ukiisha kwanza (kazi huendelea)."),
            ("subiri_yoyote(ahadi: Orodha<Ahadi<Haijulikani>>) -> Jozi<Namba, Haijulikani>", "Subiri ya kwanza kumaliza kati ya kazi hizo: (nafasi yake, thamani yake)."),
            ("subiri_zote(ahadi: Orodha<Ahadi<Haijulikani>>) -> Orodha<Haijulikani>", "Subiri kazi zote; thamani zao kwa mpangilio (kosa la kwanza likitokea, hilo)."),
            ("fungo(thamani: Haijulikani) -> Tokeo<Fungo<Haijulikani>, Neno>", "Kufuli inayolinda thamani inayoshirikiwa na nyuzi."),
            ("njia() -> Jozi<NjiaTx<Haijulikani>, NjiaRx<Haijulikani>>", "Njia ya ujumbe: (mtumaji, mpokeaji)."),
            ("njia_na_kikomo(kikomo: Namba) -> Jozi<NjiaTxBounded<Haijulikani>, NjiaRxBounded<Haijulikani>>", "Njia yenye nafasi `kikomo`; kutuma husubiri ikijaa."),
            ("subiri_tenda(uzi_id: Namba) -> Tokeo<Tupu, Neno>", "Subiri uzi ulioanzishwa na `tenda` umalize."),
            ("tenda(kazi_jina: Neno, ...hoja: Haijulikani) -> Tokeo<Namba, Neno>", "Anza kazi `kazi_jina(hoja...)` kwenye uzi mpya; kitambulisho cha uzi."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "kiungo",
        doc: "Kiungo (FFI): bado haijatengenezwa; kila kazi hurudisha kosa.",
        functions: &[
            ("saza_kiungo(njia: Neno) -> Tokeo<Anuani, Neno>", "Pakia maktaba ya nje (bado haijatengenezwa)."),
            ("wito_kiungo(anuani: Anuani, jina: Neno) -> Tokeo<Haijulikani, Neno>", "Ita kazi ya maktaba ya nje (bado haijatengenezwa)."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "syscall",
        doc: "Syscall: wito wa moja kwa moja kwa kernel (ndani ya `wazi`).",
        functions: &[("syscall(nr: Namba, a: Namba, b: Namba, c: Namba) -> Anuani", "Wito wa kernel nambari nr wenye hoja tatu.")],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "runtime",
        doc: "Runtime: habari kuhusu kitekelezi na mfumo kinachoendeshwa.",
        functions: &[
            ("arch() -> Neno", "Usanifu wa CPU (\"x86_64\", \"aarch64\", \"wasm32\")."),
            ("jina_os() -> Neno", "Jina la mfumo wa uendeshaji."),
            ("mazingira() -> Kamusi<Neno, Neno>", "Vigezo vyote vya mazingira."),
            ("muda_wa_kuanza() -> Wakati", "Wakati programu ilipoanza."),
            ("ni_debug() -> Ukweli", "Kweli kwa ujenzi wa debug."),
            ("ni_wasm() -> Ukweli", "Kweli ndani ya kivinjari (wasm)."),
            ("toleo() -> Neno", "Toleo la Asili."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "kasha_gc",
        doc: "Kasha_GC: thamani inayoshirikiwa kwa kuhesabu marejeo. Ya hiari: inahitaji `leta kasha_gc`. Njia (.shirikisha, .idadi, .pata, .weka) ni za kishikizo.",
        functions: &[
            ("kasha_gc_dhaifu(kasha: Kasha_GC<Haijulikani>) -> Kasha_GC_Dhaifu<Haijulikani>", "Rejeo dhaifu lisilohesabiwa; huvunja mizunguko."),
            ("kasha_gc_unda(thamani: Haijulikani) -> Kasha_GC<Haijulikani>", "Kasha jipya linaloshikilia thamani."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "ruwaza",
        doc: "Ruwaza: sintaksia ya Perl (Unicode, bila look-around wala backreferences); muda wa kulinganisha unakua kwa mstari na urefu wa maandishi. Kila kazi hurudisha kosa ruwaza isipokuwa sahihi.",
        functions: &[
            ("ruwaza_badilisha(ruwaza: Neno, maandishi: Neno, kwa: Neno) -> Tokeo<Neno, Neno>", "Badilisha kila ulinganisho; `$1` au `${jina}` ndani ya `kwa` ni kikundi, `$$` ni alama ya dola."),
            ("ruwaza_gawanya(ruwaza: Neno, maandishi: Neno) -> Tokeo<Orodha<Neno>, Neno>", "Gawanya maandishi kwenye kila ulinganisho."),
            ("ruwaza_inalingana(ruwaza: Neno, maandishi: Neno) -> Tokeo<Ukweli, Neno>", "Kweli ruwaza ikilingana mahali popote."),
            ("ruwaza_tafuta(ruwaza: Neno, maandishi: Neno) -> Tokeo<Neno?, Neno>", "Ulinganisho wa kwanza."),
            ("ruwaza_vikundi(ruwaza: Neno, maandishi: Neno) -> Tokeo<Orodha<Neno>?, Neno>", "Vikundi vya ulinganisho wa kwanza; kikundi 0 ni ulinganisho mzima, kisichoshiriki ni maandishi tupu."),
            ("ruwaza_zote(ruwaza: Neno, maandishi: Neno) -> Tokeo<Orodha<Neno>, Neno>", "Kila ulinganisho, kwa mpangilio."),
        ],
        constants: &[],
        structs: &[],
    },
    BuiltinModule {
        name: "usimbaji",
        doc: "Usimbaji: base64, hex, hashi na vitambulisho (juu ya baiti za UTF-8; hex ni herufi ndogo).",
        functions: &[
            ("base64_fumbua(maandishi: Neno) -> Tokeo<Neno, Neno>", "Fumbua base64 ya kawaida; kosa isipokuwa base64 ya maandishi ya UTF-8."),
            ("base64_fumbua_baiti(maandishi: Neno) -> Tokeo<Baiti, Neno>", "Fumbua base64 ya kawaida kuwa baiti."),
            ("base64_simba(maandishi: Neno) -> Neno", "Simba kwa base64 ya kawaida."),
            ("hashi_sha256(maandishi: Neno) -> Neno", "SHA-256, kwa hex."),
            ("hashi_sha512(maandishi: Neno) -> Neno", "SHA-512, kwa hex."),
            ("hex_fumbua(maandishi: Neno) -> Tokeo<Baiti, Neno>", "Baiti kutoka hex (herufi kubwa au ndogo)."),
            ("hex_simba(maandishi: Neno) -> Neno", "Baiti kwa hex."),
            ("hmac_sha256(ufunguo: Neno, ujumbe: Neno) -> Neno", "HMAC-SHA256 ya ujumbe kwa ufunguo, kwa hex."),
            ("kitambulisho() -> Neno", "UUID ya nasibu (toleo 4); `nasibu_mbegu` huifanya irudiwe pia."),
        ],
        constants: &[],
        structs: &[],
    },
];

/// A field of a builtin `umbo`: its name, its type as written, whether a literal may omit it,
/// and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: String,
    pub optional: bool,
    pub doc: &'static str,
}

/// A `umbo` builtins take or return (`OmbiHttp`, `JibuHttp`, `ChaguoHttp`), known to every
/// program without a declaration; a program's own `umbo` of the same name takes its place.
#[derive(Clone, Debug)]
pub struct BuiltinStruct {
    pub name: String,
    pub module: &'static str,
    pub fields: Vec<StructField>,
    pub doc: &'static str,
}

fn parse_struct(src: &StructSrc, module: &'static str) -> BuiltinStruct {
    let fields = src
        .fields
        .iter()
        .map(|(f, doc)| {
            let (n, ty) = f.split_once(':').expect("uga: `jina: Aina`");
            let n = n.trim();
            StructField {
                name: n.trim_end_matches('?').to_string(),
                ty: ty.trim().to_string(),
                optional: n.ends_with('?'),
                doc,
            }
        })
        .collect();
    BuiltinStruct {
        name: src.name.to_string(),
        module,
        fields,
        doc: src.doc,
    }
}

/// Every builtin `umbo`, in table order.
pub fn builtin_structs() -> &'static [BuiltinStruct] {
    static STRUCTS: OnceLock<Vec<BuiltinStruct>> = OnceLock::new();
    STRUCTS.get_or_init(|| {
        BUILTIN_MODULES
            .iter()
            .flat_map(|m| m.structs.iter().map(|s| parse_struct(s, m.name)))
            .collect()
    })
}

/// The builtin `umbo` called `name`, if there is one.
pub fn builtin_struct(name: &str) -> Option<&'static BuiltinStruct> {
    builtin_structs().iter().find(|s| s.name == name)
}

/// Names the builtin `umbo` types for [`crate::semantic::parse_value_type_with`].
pub fn resolve_builtin_type(name: &str) -> Option<ValueType> {
    builtin_struct(name).map(|s| ValueType::Struct(s.name.clone()))
}

/// Parse a signature written as in [`BUILTIN_MODULES`] (or an `.asi` line):
/// `[kazi ]name[<T>](p: T, q?: T, ...r: T) [-> R]`. `None` when it is not one.
pub fn parse_signature(
    src: &str,
    resolve: &dyn Fn(&str) -> Option<ValueType>,
) -> Option<(String, FnContract)> {
    let src = src.trim();
    let src = src.strip_prefix("kazi ").unwrap_or(src).trim_start();
    let open = src.find('(')?;
    let close = matching_paren(src, open)?;
    let name = src[..open].split('<').next()?.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let mut contract = FnContract::default();
    for p in crate::split_generic_args(&src[open + 1..close])
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if contract.variadic {
            return None; // `...r` must be last
        }
        let (n, ty) = p.split_once(':').unwrap_or((p, "Haijulikani"));
        let mut n = n.trim();
        if let Some(rest) = n.strip_prefix("...") {
            contract.variadic = true;
            n = rest;
        } else if let Some(rest) = n.strip_suffix('?') {
            contract.optional += 1;
            n = rest;
        } else if contract.optional > 0 {
            return None; // optional parameters are trailing
        }
        contract.names.push(n.to_string());
        contract
            .params
            .push(crate::semantic::parse_value_type_with(ty.trim(), resolve));
    }
    let tail = src[close + 1..].trim();
    let tail = tail.split('{').next().unwrap_or("").trim();
    contract.ret = match tail.strip_prefix("->") {
        Some(r) => crate::semantic::parse_value_type_with(r.trim(), resolve),
        None => ValueType::Tupu,
    };
    Some((name.to_string(), contract))
}

fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `name(p: T, q?: T, ...r: T) -> R` — the inverse of [`parse_signature`] (parameters without a
/// known name are written `_0`, `_1`, …).
pub fn format_signature(name: &str, c: &FnContract) -> String {
    let last = c.params.len().saturating_sub(1);
    let first_optional = c.min_args();
    let params: Vec<String> = c
        .params
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let n = c.names.get(i).cloned().unwrap_or_else(|| format!("_{i}"));
            let ty = crate::format_value_type(t);
            if c.variadic && i == last {
                format!("...{n}: {ty}")
            } else if i >= first_optional {
                format!("{n}?: {ty}")
            } else {
                format!("{n}: {ty}")
            }
        })
        .collect();
    format!(
        "{name}({}) -> {}",
        params.join(", "),
        crate::format_value_type(&c.ret)
    )
}

fn build_table(m: &BuiltinModule) -> BuiltinExportTable {
    let mut functions = HashMap::new();
    for (src, doc) in m.functions {
        let (name, mut contract) = parse_signature(src, &resolve_builtin_type)
            .unwrap_or_else(|| panic!("{}: sahihi batili: {src}", m.name));
        contract.doc = doc.to_string();
        functions.insert(name, contract);
    }
    let constants = m
        .constants
        .iter()
        .map(|(src, _)| {
            let (name, ty) = src.split_once(':').expect("thabiti: `JINA: Aina`");
            (
                name.trim().to_string(),
                crate::semantic::parse_value_type_with(ty.trim(), &resolve_builtin_type),
            )
        })
        .collect();
    BuiltinExportTable {
        functions,
        constants,
    }
}

/// Return builtin export table for the given module name, or None.
pub fn builtin_module_exports(name: &str) -> Option<BuiltinExportTable> {
    static TABLES: OnceLock<HashMap<&'static str, BuiltinExportTable>> = OnceLock::new();
    TABLES
        .get_or_init(|| {
            BUILTIN_MODULES
                .iter()
                .map(|m| (m.name, build_table(m)))
                .collect()
        })
        .get(name)
        .cloned()
}

/// The table entry for builtin module `name`.
pub fn builtin_module(name: &str) -> Option<&'static BuiltinModule> {
    BUILTIN_MODULES.iter().find(|m| m.name == name)
}

/// `lib/std/<name>.asi`: builtin module `m` written in Asili, generated from [`BUILTIN_MODULES`]
/// (`pata-core`'s `stdlib_stubs_are_generated` test checks the files and rewrites them with
/// `ASILI_GOLDEN=write`).
pub fn render_stub(m: &BuiltinModule) -> String {
    let mut out = format!(
        "# Kiolesura cha moduli ya `{}`, kwa Asili.\n\
         # Imetengenezwa kutoka core/parser/src/builtins.rs (BUILTIN_MODULES) — usihariri faili hili;\n\
         # badilisha jedwali, kisha: ASILI_GOLDEN=write cargo test -p pata-core stdlib_stubs\n",
        m.name
    );
    let comment = |out: &mut String, text: &str| {
        for line in wrap(text, 94) {
            out.push_str("# ");
            out.push_str(&line);
            out.push('\n');
        }
    };
    if !m.doc.is_empty() {
        out.push_str("#\n");
        comment(&mut out, m.doc);
    }
    for st in builtin_structs().iter().filter(|s| s.module == m.name) {
        out.push('\n');
        comment(&mut out, st.doc);
        out.push_str(&format!("umbo {} {{\n", st.name));
        for f in &st.fields {
            for line in wrap(f.doc, 90) {
                out.push_str(&format!("    # {line}\n"));
            }
            let opt = if f.optional { "?" } else { "" };
            out.push_str(&format!("    {}{opt}: {},\n", f.name, f.ty));
        }
        out.push_str("}\n");
    }
    if !m.constants.is_empty() {
        out.push('\n');
        for (src, doc) in m.constants {
            comment(&mut out, doc);
            out.push_str(&format!("thabiti {src}\n"));
        }
    }
    let table = builtin_module_exports(m.name).unwrap_or_default();
    for (src, _) in m.functions {
        let Some((name, _)) = parse_signature(src, &resolve_builtin_type) else {
            continue;
        };
        let contract = &table.functions[&name];
        out.push('\n');
        comment(&mut out, &contract.doc);
        out.push_str(&format!("kazi {}\n", format_signature(&name, contract)));
    }
    out
}

/// `text` in lines of at most `width` characters, broken at spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
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
    Baiti,
    Ahadi,
}

/// Built-in methods that only read their receiver, per [`MethodReceiver`]. The one list: the
/// semantic analyzer accepts exactly these (with [`MUTATING_METHODS`] and
/// [`CALLBACK_METHODS`]), and both engines dispatch on them.
pub const PURE_METHODS: &[&[&str]] = &[
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
        "baiti",
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
    &["soma", "andika", "funga", "soma_baiti"],
    &["soma", "andika", "funga", "soma_bailisi", "soma_baiti"],
    &["pata"],
    &["tuma"],
    &["pokea"],
    &["funga", "fungua", "pata", "weka"],
    &[
        "clona",
        "urefu",
        "tupu",
        "kata",
        "tafuta",
        "ina",
        "anza_na",
        "maliza_na",
        "gawanya",
        "geuza",
        "kwa_neno",
        "kwa_orodha",
        "hex",
        "base64",
        "hashi_sha256",
        "hashi_sha512",
        "soma_nambari",
    ],
    &["imekwisha", "ghairi"],
];

/// Built-in methods that change their receiver in place, per [`MethodReceiver`].
pub const MUTATING_METHODS: &[&[&str]] = &[
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
    &[],
    &[],
];

/// Built-in methods that call a `kazi` (or builtin) named by their first argument, per element.
pub const CALLBACK_METHODS: &[&[&str]] = &[
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
    &[],
    &[],
];

/// Whether `receiver` has a built-in method `name`.
pub fn has_builtin_method(receiver: MethodReceiver, name: &str) -> bool {
    let i = receiver as usize;
    [PURE_METHODS, MUTATING_METHODS, CALLBACK_METHODS]
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
        (ValueType::Neno, "baiti") => ValueType::Baiti,
        (ValueType::Baiti, "clona" | "kata" | "geuza") => ValueType::Baiti,
        (ValueType::Baiti, "urefu") => ValueType::Namba,
        (ValueType::Baiti, "tupu" | "ina" | "anza_na" | "maliza_na") => ValueType::Ukweli,
        (ValueType::Baiti, "tafuta") => ValueType::Chaguo(Box::new(ValueType::Namba)),
        (ValueType::Baiti, "gawanya") => ValueType::Orodha(Box::new(ValueType::Baiti)),
        (ValueType::Baiti, "kwa_neno") => {
            ValueType::Tokeo(Box::new(ValueType::Neno), Box::new(ValueType::Neno))
        }
        (ValueType::Baiti, "kwa_orodha") => ValueType::Orodha(Box::new(ValueType::Namba)),
        (ValueType::Baiti, "hex" | "base64" | "hashi_sha256" | "hashi_sha512") => ValueType::Neno,
        (ValueType::Ahadi(_), "imekwisha") => ValueType::Ukweli,
        (ValueType::Ahadi(_), "ghairi") => ValueType::Tupu,
        (ValueType::Baiti, "soma_nambari") => ValueType::Chaguo(Box::new(ValueType::Namba)),
        (ValueType::Faili | ValueType::Mkondo, "soma_baiti") => {
            ValueType::Tokeo(Box::new(ValueType::Baiti), Box::new(ValueType::Neno))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(src: &str) -> Option<(String, FnContract)> {
        parse_signature(src, &resolve_builtin_type)
    }

    #[test]
    fn signatures_parse_and_print_back() {
        for src in [
            "f(a: Neno, b?: Kamusi<Neno, Neno>) -> Tokeo<JibuHttp, Neno>",
            "g(...vitu: Haijulikani) -> Orodha<Haijulikani>",
            "h(a: Namba, ...b: Neno) -> Tupu",
            "k() -> Namba?",
        ] {
            let (name, c) = sig(src).expect(src);
            assert_eq!(format_signature(&name, &c), src);
        }
        let (_, c) = sig("kazi f(a: Neno, b?: Namba, c?: Neno) -> Tupu").unwrap();
        assert_eq!((c.min_args(), c.optional, c.variadic), (1, 2, false));
        assert!(c.accepts(1) && c.accepts(3) && !c.accepts(0) && !c.accepts(4));
        let (_, c) = sig("tenda(kazi_jina: Neno, ...hoja: Haijulikani) -> Tupu").unwrap();
        assert!(c.accepts(1) && c.accepts(9) && !c.accepts(0));
        assert_eq!(c.param_for_arg(5), Some(&ValueType::Unknown));
        // A builtin `umbo` is a type; no `->` is `Tupu`.
        let (_, c) = sig("kazi f(o: OmbiHttp)").unwrap();
        assert_eq!(c.params, vec![ValueType::Struct("OmbiHttp".into())]);
        assert_eq!(c.ret, ValueType::Tupu);
    }

    #[test]
    fn malformed_signatures_are_refused() {
        assert!(
            sig("f(a?: Neno, b: Neno) -> Tupu").is_none(),
            "optional before required"
        );
        assert!(
            sig("f(...a: Neno, b: Neno) -> Tupu").is_none(),
            "variadic not last"
        );
        assert!(sig("f a: Neno) -> Tupu").is_none());
        assert!(sig("(a: Neno) -> Tupu").is_none());
    }

    /// Every table entry parses, every builtin `umbo` field has a type, and the module list
    /// matches [`BUILTIN_MODULE_NAMES`].
    #[test]
    fn the_table_is_well_formed() {
        let names: Vec<&str> = BUILTIN_MODULES.iter().map(|m| m.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        let mut expected = BUILTIN_MODULE_NAMES.to_vec();
        expected.sort();
        assert_eq!(sorted, expected);
        for m in BUILTIN_MODULES {
            let table = builtin_module_exports(m.name).unwrap();
            assert_eq!(
                table.functions.len(),
                m.functions.len(),
                "{} repeats a name",
                m.name
            );
            for (src, doc) in m.functions {
                assert!(!doc.is_empty(), "{}: `{src}` has no description", m.name);
            }
        }
        for st in builtin_structs() {
            assert!(
                st.fields
                    .iter()
                    .all(|f| !f.ty.is_empty() && !f.doc.is_empty()),
                "{}",
                st.name
            );
        }
    }
}
