leta hisabati
leta mfumo
leta matumizi

thabiti LABEL_HAMNA: Neno = "Hamna"
thabiti LABEL_SOME: Neno = "Some"
thabiti HERUFI_A: Neno = "a"
thabiti HERUFI_X: Neno = "x"
thabiti HERUFI_E_ACCENT: Neno = "é"
thabiti FUNGUO_HAIPO: Neno = "z"
thabiti NENO_HELLO: Neno = "hello"

umbo Pika { x: Namba, y: Neno }

# Basic addition.
kazi namba_basic() -> Namba {
  rejesha 3 + 5
}

# The `Ukomo` (max-value) numeric constant.
kazi namba_ukomo() -> Namba {
  rejesha Ukomo
}

# The `Siyo_Namba` (NaN) numeric constant.
kazi namba_siyo_namba() -> Namba {
  rejesha Siyo_Namba
}

# Negation of `Ukomo`.
kazi namba_neg_ukomo() -> Namba {
  rejesha - Ukomo
}

# NaN is never equal to itself.
kazi namba_nan_eq() -> Ukweli {
  rejesha Siyo_Namba == Siyo_Namba
}

# `jaribu` unwrapping a successful Biti8 cast.
kazi chaguo_some() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

# Looking up a missing Kamusi key returns `Hamna` via `linganisha`.
kazi chaguo_none_as_neno() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza(HERUFI_A, 1)
  weka val = m.pata(FUNGUO_HAIPO)
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# A basic string literal.
kazi neno_basic() -> Neno {
  rejesha NENO_HELLO
}

# The empty string.
kazi neno_empty() -> Neno {
  rejesha ""
}

# `.urefu()` (length, in graphemes) of a string.
kazi neno_urefu(s: Neno) -> Namba {
  rejesha s.urefu()
}

# `.biti_ngapi()` (length, in bytes) of a string.
kazi neno_biti_ngapi(s: Neno) -> Namba {
  rejesha s.biti_ngapi()
}

# Boolean AND.
kazi ukweli_basic() -> Ukweli {
  rejesha kweli na si_kweli
}

# Double logical negation.
kazi ukweli_double_not() -> Ukweli {
  rejesha siyo siyo kweli
}

# Casting a number to a boolean.
kazi ukweli_from_namba() -> Ukweli {
  rejesha 0 kama Ukweli
}

# A basic character literal.
kazi herufi_basic() -> Herufi {
  rejesha 'a'
}

# Casting a number (codepoint) to a character.
kazi herufi_from_namba() -> Herufi {
  rejesha 65 kama Herufi
}

# Casting a character to a string.
kazi herufi_to_neno() -> Neno {
  rejesha ('A' kama Neno)
}

# Length of a non-empty Orodha (list).
kazi orodha_basic() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a.urefu()
}

# Indexing into an Orodha with `?` propagation.
kazi orodha_index() -> Namba {
  weka a = orodha(5, 15, 25)
  rejesha a[1]?
}

# Length of an empty Orodha.
kazi orodha_empty() -> Namba {
  weka a = orodha()
  rejesha a.urefu()
}

# Kamusi (map) insert then lookup.
kazi kamusi_basic() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(HERUFI_X, 42)
  rejesha m.pata(HERUFI_X) kama Namba
}

# Looking up a missing Kamusi key via `linganisha`.
kazi kamusi_missing() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza(HERUFI_A, 1)
  weka val = m.pata(FUNGUO_HAIPO)
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# `linganisha` directly on a `Hamna` value.
kazi hamna_match() -> Neno {
  weka val = Hamna
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha "other" } }
}

# Jozi (pair) construction and access via `.kwanza()`/`.pili()`.
kazi jozi_basic() -> Namba {
  weka p = jozi(7, 8)
  rejesha ((p.kwanza()) kama Namba) + ((p.pili()) kama Namba)
}

# A Jozi mixing a Neno and a Ukweli.
kazi jozi_neno_ukweli() -> Neno {
  weka p = jozi("key", kweli)
  rejesha p.kwanza()
}

# `jaribu` unwrapping a successful division.
kazi tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

# Basic struct construction and field access.
kazi struct_basic() -> Namba {
  weka p = Pika { x: 3, y: "hi" }
  rejesha p.x
}

# Accessing a struct's Neno field.
kazi struct_neno() -> Neno {
  weka p = Pika { x: 1, y: "world" }
  rejesha p.y
}

# Casting `Hamna` to a number.
kazi hamna_kama_namba() -> Namba {
  rejesha Hamna kama Namba
}

# `.ongeza()` with no argument appends `Hamna`.
kazi orodha_with_hamna() -> Namba {
  weka a = orodha(1, 2)
  a.ongeza()
  rejesha a.urefu()
}

# Grapheme-based length of a multi-byte (accented) character.
kazi neno_grapheme() -> Namba {
  weka s = HERUFI_E_ACCENT
  rejesha s.urefu()
}

# Unary negation binds tighter than addition.
kazi namba_precedence() -> Namba {
  rejesha - 1 + 10
}

# Byte-slicing a multi-byte character with `.kata()`.
kazi neno_kata_byte() -> Neno {
  weka s = HERUFI_E_ACCENT
  rejesha s.kata(0, 2)
}

# Byte length (not grapheme length) of a multi-byte character.
kazi neno_e_biti_ngapi() -> Namba {
  weka s = HERUFI_E_ACCENT
  rejesha s.biti_ngapi()
}

# `.tafuta()` (find) for a substring that isn't present.
kazi neno_tafuta_missing() -> Neno {
  weka s = NENO_HELLO
  weka val = s.tafuta(HERUFI_X)
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# Casting `Hamna` to a boolean.
kazi ukweli_hamna_cast() -> Ukweli {
  rejesha Hamna kama Ukweli
}

# `.ondoa()` (remove) with an out-of-bounds index.
kazi orodha_ondoa_oob() -> Neno {
  weka a = orodha(10, 20)
  weka val = a.ondoa(10)
  linganisha val {
    Hamna => { rejesha "Chaguo(None)" }
    _ => { rejesha LABEL_SOME } }
}

# A Jozi whose first element is `Hamna`.
kazi jozi_with_hamna() -> Namba {
  weka p = jozi(Hamna, 7)
  rejesha (p.pili()) kama Namba
}

# Casting the number 0 to a character (NUL).
kazi herufi_zero() -> Herufi {
  rejesha 0 kama Herufi
}

# Casting a struct value to a string.
kazi struct_kama_neno() -> Neno {
  weka p = Pika { x: 1, y: HERUFI_A }
  rejesha p kama Neno
}

# Length of an Orodha of struct values.
kazi orodha_of_struct_urefu() -> Namba {
  weka a = orodha()
  a.ongeza(Pika { x: 1, y: "first" })
  a.ongeza(Pika { x: 2, y: "second" })
  rejesha a.urefu()
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- All types: basic + push ---")
  chapisha("Namba: 3+5 = " + (namba_basic() kama Neno))
  chapisha("Namba: Ukomo = " + (namba_ukomo() kama Neno))
  chapisha("Namba: Siyo_Namba = " + (namba_siyo_namba() kama Neno))
  chapisha("Namba: -Ukomo = " + (namba_neg_ukomo() kama Neno))
  chapisha("Push: Siyo_Namba==Siyo_Namba = " + (namba_nan_eq() kama Neno))
  chapisha("Chaguo: jaribu (100 kama Biti8) = " + (chaguo_some() kama Neno))
  chapisha("Chaguo: 1000 kama Biti8 => " + chaguo_none_as_neno())
  chapisha("Neno: hello, empty urefu = " + (neno_urefu(neno_empty()) kama Neno))
  chapisha("Neno: é urefu = " + (neno_grapheme() kama Neno))
  chapisha("Ukweli: kweli na si_kweli = " + (ukweli_basic() kama Neno))
  chapisha("Ukweli: siyo siyo kweli = " + (ukweli_double_not() kama Neno))
  chapisha("Ukweli: 0 kama Ukweli = " + (ukweli_from_namba() kama Neno))
  chapisha("Herufi: 65 kama Herufi => " + herufi_to_neno())
  chapisha("Orodha: urefu = " + (orodha_basic() kama Neno))
  chapisha("Orodha: a[1] = " + (orodha_index() kama Neno))
  chapisha("Orodha: empty urefu = " + (orodha_empty() kama Neno))
  chapisha("Orodha: after ongeza() no arg, urefu = " + (orodha_with_hamna() kama Neno))
  chapisha("Kamusi: pata(x) = " + (kamusi_basic() kama Neno))
  chapisha("Kamusi: pata(z) missing => " + kamusi_missing())
  chapisha("Jozi: kwanza+pili = " + (jozi_basic() kama Neno))
  chapisha("Jozi: (key, kweli) => " + jozi_neno_ukweli())
  chapisha("Tokeo: jaribu gawio(10,2) = " + (tokeo_ok() kama Neno))
  chapisha("Struct: Pika.x = " + (struct_basic() kama Neno))
  chapisha("Struct: Pika.y = " + struct_neno())
  chapisha("Hamna: match => " + hamna_match())
  chapisha("Hamna: Hamna kama Namba = " + (hamna_kama_namba() kama Neno))
  chapisha("Push: -1+10 = " + (namba_precedence() kama Neno))
  chapisha("--- Push further ---")
  chapisha("Neno: é.kata(0,2) = " + neno_kata_byte())
  chapisha("Neno: é.biti_ngapi (bytes) = " + (neno_e_biti_ngapi() kama Neno))
  chapisha("Orodha: a[10] oob => Tokeo(Err) in code")
  chapisha("Neno: tafuta(x) missing => " + neno_tafuta_missing())
  chapisha("Ukweli: Hamna kama Ukweli = " + (ukweli_hamna_cast() kama Neno))
  chapisha("Orodha: ondoa(10) oob => " + orodha_ondoa_oob())
  chapisha("Jozi: jozi(Hamna, 7).pili = " + (jozi_with_hamna() kama Neno))
  chapisha("Herufi: 0 kama Herufi urefu = " + ((herufi_zero() kama Neno).urefu() kama Neno))
  chapisha("Struct: Pika kama Neno = " + struct_kama_neno())
  chapisha("Orodha: orodha of Pika, urefu = " + (orodha_of_struct_urefu() kama Neno))
  chapisha("--- mwisho ---")
}
