leta hisabati
leta mfumo
leta matumizi

umbo Pika { x: Namba, y: Neno }

kazi struct_pass_move(p: Pika) -> Namba {
  rejesha p.x
}

kazi orodha_empty() -> Namba {
  weka a = orodha()
  rejesha a.urefu()
}

kazi orodha_single() -> Namba {
  weka a = orodha(42)
  rejesha a[0]?
}

kazi orodha_index_first() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a[0]?
}

kazi orodha_index_last() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a[2]?
}

kazi orodha_negative_index() -> Namba {
  weka a = orodha(7, 8, 9)
  weka i = 0 - 1
  rejesha a[i]?
}

kazi orodha_urefu_after_ongeza() -> Namba {
  weka a = orodha(1, 2)
  a.ongeza(3)
  a.ongeza(4)
  rejesha a.urefu()
}

kazi orodha_ongeza_hamna() -> Namba {
  weka a = orodha(1)
  a.ongeza()
  rejesha a.urefu()
}

kazi orodha_ondoa_first() -> Namba {
  weka a = orodha(100, 200, 300)
  weka v = a.ondoa(0)
  rejesha (v kama Namba)
}

kazi orodha_ondoa_middle() -> Namba {
  weka a = orodha(5, 15, 25)
  weka v = a.ondoa(1)
  rejesha (v kama Namba)
}

kazi orodha_ondoa_last() -> Namba {
  weka a = orodha(1, 2, 3)
  weka v = a.ondoa(2)
  rejesha (v kama Namba)
}

kazi orodha_ondoa_oob() -> Neno {
  weka a = orodha(10, 20)
  weka val = a.ondoa(99)
  linganisha val {
    Hamna => { rejesha "Chaguo(None)" }
    _ => { rejesha "Some" } }
}

kazi orodha_after_ondoa_urefu() -> Namba {
  weka a = orodha(1, 2, 3)
  a.ondoa(1)
  rejesha a.urefu()
}

kazi orodha_kila_mmoja() -> Neno {
  weka a = orodha(1, 2, 3)
  a.kila_mmoja()
  rejesha "Tupu"
}

kazi orodha_of_struct_urefu() -> Namba {
  weka a = orodha()
  a.ongeza(Pika { x: 1, y: "a" })
  a.ongeza(Pika { x: 2, y: "b" })
  rejesha a.urefu()
}

kazi orodha_of_jozi() -> Namba {
  weka a = orodha(jozi(1, 2), jozi(3, 4))
  weka p = a[0]?
  rejesha (p.kwanza()) kama Namba
}

kazi kamusi_empty_urefu() -> Neno {
  weka m = kamusi_tupu()
  rejesha "tupu"
}

kazi kamusi_neno_key() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza("x", 100)
  rejesha m.pata("x") kama Namba
}

kazi kamusi_namba_key() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza(42, "jibu")
  rejesha m.pata(42) kama Neno
}

kazi kamusi_ukweli_key() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza(kweli, 1)
  m.ingiza(si_kweli, 0)
  rejesha (m.pata(kweli) kama Namba) + (m.pata(si_kweli) kama Namba)
}

kazi kamusi_herufi_key() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza('A', "Herufi")
  rejesha m.pata('A') kama Neno
}

kazi kamusi_missing() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza("a", 1)
  weka val = m.pata("z")
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi kamusi_value_orodha() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza("lst", orodha(5, 10, 15))
  linganisha m.pata("lst") {
    Hamna => { rejesha 0 }
    _ => { rejesha 3 }
  }
}

kazi jozi_basic() -> Namba {
  weka p = jozi(7, 8)
  rejesha ((p.kwanza()) kama Namba) + ((p.pili()) kama Namba)
}

kazi jozi_nested() -> Namba {
  weka p = jozi(jozi(10, 20), 30)
  weka inner = p.kwanza()
  rejesha ((inner.kwanza()) kama Namba) + ((inner.pili()) kama Namba) + ((p.pili()) kama Namba)
}

kazi jozi_hamna_element() -> Namba {
  weka p = jozi(Hamna, 7)
  rejesha (p.pili()) kama Namba
}

kazi jozi_neno_ukweli() -> Neno {
  weka p = jozi("key", kweli)
  rejesha p.kwanza()
}

kazi struct_literal() -> Namba {
  weka s = Pika { x: 3, y: "hi" }
  rejesha s.x
}

kazi struct_field_neno() -> Neno {
  weka s = Pika { x: 1, y: "world" }
  rejesha s.y
}

kazi struct_rebuild() -> Namba {
  weka s = Pika { x: 10, y: "old" }
  weka s2 = Pika { x: s.x + 1, y: "new" }
  rejesha s2.x
}

kazi struct_kama_neno() -> Neno {
  weka p = Pika { x: 1, y: "a" }
  rejesha p kama Neno
}

kazi chaguo_hamna_match() -> Neno {
  weka val = Hamna
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi chaguo_some_cast() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

kazi chaguo_au_namba_some() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza("x", 42)
  weka c = m.pata("x")
  rejesha c.angu(0)
}

kazi tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

kazi tokeo_propagate_in_bounds() -> Namba {
  weka a = orodha(5, 15, 25)
  rejesha a[1]?
}

kazi neno_empty_urefu() -> Namba {
  weka s = ""
  rejesha s.urefu()
}

kazi neno_empty_biti_ngapi() -> Namba {
  weka s = ""
  rejesha s.biti_ngapi()
}

kazi neno_kata_zero_zero() -> Neno {
  weka s = "hello"
  rejesha s.kata(0, 0)
}

kazi neno_kata_slice() -> Neno {
  weka s = "hello"
  rejesha s.kata(1, 4)
}

kazi neno_tafuta_found() -> Namba {
  weka s = "hello"
  weka c = s.tafuta("ell")
  rejesha c kama Namba
}

kazi neno_tafuta_missing() -> Neno {
  weka s = "hi"
  weka val = s.tafuta("x")
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi neno_unganisha() -> Neno {
  weka s = "a"
  rejesha s.unganisha("-")
}

kazi neno_grapheme_urefu() -> Namba {
  weka s = "é"
  rejesha s.urefu()
}

kazi neno_byte_kata() -> Neno {
  weka s = "é"
  rejesha s.kata(0, 2)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- Orodha limits ---")
  chapisha("empty urefu: " + (orodha_empty() kama Neno))
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
  chapisha("empty urefu: " + (neno_empty_urefu() kama Neno))
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
