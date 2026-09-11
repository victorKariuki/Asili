leta hisabati
leta mfumo
leta matumizi

thabiti FUNGUO_X: Neno = "x"
thabiti HERUFI_A: Neno = "a"
thabiti LABEL_HAMNA: Neno = "Hamna"
thabiti LABEL_SOME: Neno = "Some"
thabiti NENO_HELLO: Neno = "hello"
thabiti NENO_HI: Neno = "hi"
thabiti FUNGUO_LST: Neno = "lst"
thabiti HERUFI_E_ACCENT: Neno = "é"
thabiti LABEL_EMPTY_UREFU: Neno = "empty urefu: "

umbo Pika { x: Namba, y: Neno }

# A struct argument is passed (moved) into a function.
kazi struct_pass_move(p: Pika) -> Namba {
  rejesha p.x
}

# Length of an empty Orodha.
kazi orodha_empty() -> Namba {
  weka a = orodha()
  rejesha a.urefu()
}

# A single-element Orodha, indexed with `?`.
kazi orodha_single() -> Namba {
  weka a = orodha(42)
  rejesha a[0]?
}

# Indexing the first element.
kazi orodha_index_first() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a[0]?
}

# Indexing the last element.
kazi orodha_index_last() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a[2]?
}

# Indexing with a negative index.
kazi orodha_negative_index() -> Namba {
  weka a = orodha(7, 8, 9)
  weka i = 0 - 1
  rejesha a[i]?
}

# Length after two `.ongeza()` (push) calls.
kazi orodha_urefu_after_ongeza() -> Namba {
  weka a = orodha(1, 2)
  a.ongeza(3)
  a.ongeza(4)
  rejesha a.urefu()
}

# `.ongeza()` with no argument appends `Hamna`.
kazi orodha_ongeza_hamna() -> Namba {
  weka a = orodha(1)
  a.ongeza()
  rejesha a.urefu()
}

# `.ondoa()` (remove) the first element.
kazi orodha_ondoa_first() -> Namba {
  weka a = orodha(100, 200, 300)
  weka v = a.ondoa(0)
  rejesha (v kama Namba)
}

# `.ondoa()` a middle element.
kazi orodha_ondoa_middle() -> Namba {
  weka a = orodha(5, 15, 25)
  weka v = a.ondoa(1)
  rejesha (v kama Namba)
}

# `.ondoa()` the last element.
kazi orodha_ondoa_last() -> Namba {
  weka a = orodha(1, 2, 3)
  weka v = a.ondoa(2)
  rejesha (v kama Namba)
}

# `.ondoa()` with an out-of-bounds index.
kazi orodha_ondoa_oob() -> Neno {
  weka a = orodha(10, 20)
  weka val = a.ondoa(99)
  linganisha val {
    Hamna => { rejesha "Chaguo(None)" }
    _ => { rejesha LABEL_SOME } }
}

# Length after removing an element.
kazi orodha_after_ondoa_urefu() -> Namba {
  weka a = orodha(1, 2, 3)
  a.ondoa(1)
  rejesha a.urefu()
}

# `.kila_mmoja()` (for-each) with no callback is a no-op.
kazi orodha_kila_mmoja() -> Neno {
  weka a = orodha(1, 2, 3)
  a.kila_mmoja()
  rejesha "Tupu"
}

# Length of an Orodha of struct values.
kazi orodha_of_struct_urefu() -> Namba {
  weka a = orodha()
  a.ongeza(Pika { x: 1, y: HERUFI_A })
  a.ongeza(Pika { x: 2, y: "b" })
  rejesha a.urefu()
}

# An Orodha of Jozi (pairs), indexed and destructured.
kazi orodha_of_jozi() -> Namba {
  weka a = orodha(jozi(1, 2), jozi(3, 4))
  weka p = a[0]?
  rejesha (p.kwanza()) kama Namba
}

# An empty Kamusi (map).
kazi kamusi_empty_urefu() -> Neno {
  weka m = kamusi_tupu()
  rejesha "tupu"
}

# A Kamusi keyed by Neno.
kazi kamusi_neno_key() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(FUNGUO_X, 100)
  rejesha m.pata(FUNGUO_X) kama Namba
}

# A Kamusi keyed by Namba.
kazi kamusi_namba_key() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza(42, "jibu")
  rejesha m.pata(42) kama Neno
}

# A Kamusi keyed by Ukweli (boolean) — both possible keys.
kazi kamusi_ukweli_key() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(kweli, 1)
  m.ingiza(si_kweli, 0)
  rejesha (m.pata(kweli) kama Namba) + (m.pata(si_kweli) kama Namba)
}

# A Kamusi keyed by Herufi (character).
kazi kamusi_herufi_key() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza('A', "Herufi")
  rejesha m.pata('A') kama Neno
}

# Looking up a missing Kamusi key via `linganisha`.
kazi kamusi_missing() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza(HERUFI_A, 1)
  weka val = m.pata("z")
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# A Kamusi value that is itself an Orodha.
kazi kamusi_value_orodha() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(FUNGUO_LST, orodha(5, 10, 15))
  linganisha m.pata(FUNGUO_LST) {
    Hamna => { rejesha 0 }
    _ => { rejesha 3 }
  }
}

# Jozi (pair) construction and access.
kazi jozi_basic() -> Namba {
  weka p = jozi(7, 8)
  rejesha ((p.kwanza()) kama Namba) + ((p.pili()) kama Namba)
}

# A Jozi nested inside another Jozi.
kazi jozi_nested() -> Namba {
  weka p = jozi(jozi(10, 20), 30)
  weka inner = p.kwanza()
  rejesha ((inner.kwanza()) kama Namba) + ((inner.pili()) kama Namba) + ((p.pili()) kama Namba)
}

# A Jozi whose first element is `Hamna`.
kazi jozi_hamna_element() -> Namba {
  weka p = jozi(Hamna, 7)
  rejesha (p.pili()) kama Namba
}

# A Jozi mixing a Neno and a Ukweli.
kazi jozi_neno_ukweli() -> Neno {
  weka p = jozi("key", kweli)
  rejesha p.kwanza()
}

# Basic struct construction and field access.
kazi struct_literal() -> Namba {
  weka s = Pika { x: 3, y: NENO_HI }
  rejesha s.x
}

# Accessing a struct's Neno field.
kazi struct_field_neno() -> Neno {
  weka s = Pika { x: 1, y: "world" }
  rejesha s.y
}

# Building a new struct value from an existing one's field.
kazi struct_rebuild() -> Namba {
  weka s = Pika { x: 10, y: "old" }
  weka s2 = Pika { x: s.x + 1, y: "new" }
  rejesha s2.x
}

# Casting a struct value to a string.
kazi struct_kama_neno() -> Neno {
  weka p = Pika { x: 1, y: HERUFI_A }
  rejesha p kama Neno
}

# `linganisha` directly on a `Hamna` value.
kazi chaguo_hamna_match() -> Neno {
  weka val = Hamna
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# `jaribu` unwrapping a successful Biti8 cast.
kazi chaguo_some_cast() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

# `.angu()` unwrapping a present Chaguo with a default fallback.
kazi chaguo_au_namba_some() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(FUNGUO_X, 42)
  weka c = m.pata(FUNGUO_X)
  rejesha c.angu(0)
}

# `jaribu` unwrapping a successful division.
kazi tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

# `?` propagation on an in-bounds Orodha index.
kazi tokeo_propagate_in_bounds() -> Namba {
  weka a = orodha(5, 15, 25)
  rejesha a[1]?
}

# Length of the empty string.
kazi neno_empty_urefu() -> Namba {
  weka s = ""
  rejesha s.urefu()
}

# Byte length of the empty string.
kazi neno_empty_biti_ngapi() -> Namba {
  weka s = ""
  rejesha s.biti_ngapi()
}

# A zero-length `.kata()` (slice) is empty.
kazi neno_kata_zero_zero() -> Neno {
  weka s = NENO_HELLO
  rejesha s.kata(0, 0)
}

# A middle slice via `.kata()`.
kazi neno_kata_slice() -> Neno {
  weka s = NENO_HELLO
  rejesha s.kata(1, 4)
}

# `.tafuta()` (find) for a substring that is present.
kazi neno_tafuta_found() -> Namba {
  weka s = NENO_HELLO
  weka c = s.tafuta("ell")
  rejesha c kama Namba
}

# `.tafuta()` for a substring that isn't present.
kazi neno_tafuta_missing() -> Neno {
  weka s = NENO_HI
  weka val = s.tafuta(FUNGUO_X)
  linganisha val {
    Hamna => { rejesha LABEL_HAMNA }
    _ => { rejesha LABEL_SOME } }
}

# `.unganisha()` (join) appends a separator.
kazi neno_unganisha() -> Neno {
  weka s = HERUFI_A
  rejesha s.unganisha("-")
}

# Grapheme-based length of a multi-byte (accented) character.
kazi neno_grapheme_urefu() -> Namba {
  weka s = HERUFI_E_ACCENT
  rejesha s.urefu()
}

# Byte-slicing a multi-byte character.
kazi neno_byte_kata() -> Neno {
  weka s = HERUFI_E_ACCENT
  rejesha s.kata(0, 2)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- Orodha limits ---")
  chapisha(LABEL_EMPTY_UREFU + (orodha_empty() kama Neno))
  chapisha("single a[0]?: " + (orodha_single() kama Neno))
  chapisha("index first: " + (orodha_index_first() kama Neno))
  chapisha("index last: " + (orodha_index_last() kama Neno))
  chapisha("negative index (clamped): " + (orodha_negative_index() kama Neno))
  chapisha("urefu after ongeza(3),(4): " + (orodha_urefu_after_ongeza() kama Neno))
  chapisha("ongeza() no arg (Hamna): " + (orodha_ongeza_hamna() kama Neno))
  chapisha("ondoa(0) first: " + (orodha_ondoa_first() kama Neno))
  chapisha("ondoa(1) middle: " + (orodha_ondoa_middle() kama Neno))
  chapisha("ondoa(2) last: " + (orodha_ondoa_last() kama Neno))
  chapisha("ondoa(99) OOB: " + orodha_ondoa_oob())
  chapisha("urefu after ondoa(1): " + (orodha_after_ondoa_urefu() kama Neno))
  chapisha("kila_mmoja returns: " + orodha_kila_mmoja())
  chapisha("orodha of Pika urefu: " + (orodha_of_struct_urefu() kama Neno))
  chapisha("--- Kamusi limits ---")
  chapisha("Neno key: " + (kamusi_neno_key() kama Neno))
  chapisha("Namba key: " + kamusi_namba_key())
  chapisha("Ukweli key sum: " + (kamusi_ukweli_key() kama Neno))
  chapisha("Herufi key: " + kamusi_herufi_key())
  chapisha("pata missing: " + kamusi_missing())
  chapisha("value orodha stored, pata => 3: " + (kamusi_value_orodha() kama Neno))
  chapisha("--- Jozi limits ---")
  chapisha("kwanza+pili: " + (jozi_basic() kama Neno))
  chapisha("nested jozi(jozi(10,20),30): " + (jozi_nested() kama Neno))
  chapisha("jozi(Hamna,7).pili: " + (jozi_hamna_element() kama Neno))
  chapisha("jozi(neno,ukweli).kwanza: " + jozi_neno_ukweli())
  chapisha("--- Struct limits ---")
  chapisha("Pika.x: " + (struct_literal() kama Neno))
  chapisha("Pika.y: " + struct_field_neno())
  chapisha("struct rebuild s2.x: " + (struct_rebuild() kama Neno))
  chapisha("struct pass (move): " + (struct_pass_move(Pika { x: 99, y: "go" }) kama Neno))
  chapisha("Pika kama Neno: " + struct_kama_neno())
  chapisha("--- Chaguo/Tokeo limits ---")
  chapisha("Hamna match: " + chaguo_hamna_match())
  chapisha("jaribu (100 kama Biti8): " + (chaguo_some_cast() kama Neno))
  chapisha("chaguo_au_namba(Some): " + (chaguo_au_namba_some() kama Neno))
  chapisha("jaribu gawio(10,2): " + (tokeo_ok() kama Neno))
  chapisha("a[1]? in bounds: " + (tokeo_propagate_in_bounds() kama Neno))
  chapisha("--- Neno limits ---")
  chapisha(LABEL_EMPTY_UREFU + (neno_empty_urefu() kama Neno))
  chapisha("empty biti_ngapi: " + (neno_empty_biti_ngapi() kama Neno))
  chapisha("kata(0,0): " + neno_kata_zero_zero())
  chapisha("kata(1,4) hello: " + neno_kata_slice())
  chapisha("tafuta(ell) index: " + (neno_tafuta_found() kama Neno))
  chapisha("tafuta(x) missing: " + neno_tafuta_missing())
  chapisha("unganisha(-): " + neno_unganisha())
  chapisha("é urefu (grapheme): " + (neno_grapheme_urefu() kama Neno))
  chapisha("é kata(0,2) bytes: " + neno_byte_kata())
  chapisha("--- mwisho ---")
}
