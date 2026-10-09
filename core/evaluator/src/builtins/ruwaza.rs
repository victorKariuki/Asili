//! Ruwaza (regular expressions): `ruwaza_inalingana`, `ruwaza_tafuta`, `ruwaza_zote`,
//! `ruwaza_vikundi`, `ruwaza_badilisha` and `ruwaza_gawanya`. The syntax is the `regex` crate's
//! (Perl-like, Unicode-aware, no look-around or backreferences), which matches in time linear in
//! the text. Each thread keeps the patterns it compiled, so a pattern used in a loop is compiled
//! once.

use std::collections::HashMap;
use std::rc::Rc;

use regex::Regex;

use super::BuiltinFn;
use crate::value::Value;

/// Most compiled patterns a thread keeps; past it, the cache starts over.
const CACHE_LIMIT: usize = 256;

thread_local! {
    static CACHE: std::cell::RefCell<HashMap<String, Rc<Regex>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// `ruwaza` compiled (from this thread's cache when it was compiled before).
fn compiled(ruwaza: &str) -> Result<Rc<Regex>, String> {
    CACHE.with(|cache| {
        if let Some(re) = cache.borrow().get(ruwaza) {
            return Ok(re.clone());
        }
        let re =
            Rc::new(Regex::new(ruwaza).map_err(|e| format!("ruwaza '{ruwaza}' si sahihi: {e}"))?);
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(ruwaza.to_string(), re.clone());
        Ok(re)
    })
}

/// Register `name(ruwaza, maandishi, …)` returning `Tokeo<_, Neno>`: `f` gets the compiled
/// pattern, the text and the remaining arguments.
fn with_pattern(
    m: &mut HashMap<String, BuiltinFn>,
    name: &'static str,
    f: fn(&Regex, &str, &[Value]) -> Value,
) {
    m.insert(
        name.to_string(),
        Box::new(move |args: &[Value]| {
            let ruwaza = super::arg_str(args, 0);
            let text = super::arg_str(args, 1);
            Ok(match compiled(&ruwaza) {
                Ok(re) => Value::sawa(f(&re, &text, args.get(2..).unwrap_or(&[]))),
                Err(e) => Value::kosa(e),
            })
        }),
    );
}

fn texts<'a>(items: impl Iterator<Item = &'a str>) -> Value {
    Value::list(items.map(|s| Value::neno(s.to_string())).collect())
}

pub(crate) fn register(m: &mut HashMap<String, BuiltinFn>) {
    with_pattern(m, "ruwaza_inalingana", |re, text, _| {
        Value::Ukweli(re.is_match(text))
    });
    with_pattern(m, "ruwaza_tafuta", |re, text, _| {
        Value::Chaguo(
            re.find(text)
                .map(|found| Box::new(Value::neno(found.as_str().to_string()))),
        )
    });
    with_pattern(m, "ruwaza_zote", |re, text, _| {
        texts(re.find_iter(text).map(|found| found.as_str()))
    });
    with_pattern(m, "ruwaza_vikundi", |re, text, _| {
        // Group 0 is the whole match; a group that took no part is empty text.
        Value::Chaguo(re.captures(text).map(|caps| {
            Box::new(texts(
                caps.iter()
                    .map(|group| group.map(|g| g.as_str()).unwrap_or("")),
            ))
        }))
    });
    with_pattern(m, "ruwaza_badilisha", |re, text, rest| {
        // `$1`, `${jina}` in the replacement name groups; `$$` is a dollar sign.
        let with = rest
            .first()
            .and_then(crate::value::as_string)
            .unwrap_or_default();
        Value::neno(re.replace_all(text, with.as_str()).into_owned())
    });
    with_pattern(m, "ruwaza_gawanya", |re, text, _| texts(re.split(text)));
}
