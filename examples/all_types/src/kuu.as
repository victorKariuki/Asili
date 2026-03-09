leta hisabati
leta mfumo
leta matumizi

umbo Pika { x: Namba, y: Neno }

kazi namba_basic() -> Namba {
  rejesha 3 + 5
}

kazi namba_ukomo() -> Namba {
  rejesha Ukomo
}

kazi namba_siyo_namba() -> Namba {
  rejesha Siyo_Namba
}

kazi namba_neg_ukomo() -> Namba {
  rejesha - Ukomo
}

kazi namba_nan_eq() -> Ukweli {
  rejesha Siyo_Namba == Siyo_Namba
}

kazi chaguo_some() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

kazi chaguo_none_as_neno() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza("a", 1)
  weka val = m.pata("z")
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi neno_basic() -> Neno {
  rejesha "hello"
}

kazi neno_empty() -> Neno {
  rejesha ""
}

kazi neno_urefu(s: Neno) -> Namba {
  rejesha s.urefu()
}

kazi neno_biti_ngapi(s: Neno) -> Namba {
  rejesha s.biti_ngapi()
}

kazi ukweli_basic() -> Ukweli {
  rejesha kweli na si_kweli
}

kazi ukweli_double_not() -> Ukweli {
  rejesha siyo siyo kweli
}

kazi ukweli_from_namba() -> Ukweli {
  rejesha 0 kama Ukweli
}

kazi herufi_basic() -> Herufi {
  rejesha 'a'
}

kazi herufi_from_namba() -> Herufi {
  rejesha 65 kama Herufi
}

kazi herufi_to_neno() -> Neno {
  rejesha ('A' kama Neno)
}

kazi orodha_basic() -> Namba {
  weka a = orodha(10, 20, 30)
  rejesha a.urefu()
}

kazi orodha_index() -> Namba {
  weka a = orodha(5, 15, 25)
  rejesha a[1]?
}

kazi orodha_empty() -> Namba {
  weka a = orodha()
  rejesha a.urefu()
}

kazi kamusi_basic() -> Namba {
  weka m = kamusi_tupu()
  m.ingiza("x", 42)
  rejesha m.pata("x") kama Namba
}

kazi kamusi_missing() -> Neno {
  weka m = kamusi_tupu()
  m.ingiza("a", 1)
  weka val = m.pata("z")
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi hamna_match() -> Neno {
  weka val = Hamna
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "other" } }
}

kazi jozi_basic() -> Namba {
  weka p = jozi(7, 8)
  rejesha ((p.kwanza()) kama Namba) + ((p.pili()) kama Namba)
}

kazi jozi_neno_ukweli() -> Neno {
  weka p = jozi("key", kweli)
  rejesha p.kwanza()
}

kazi tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

kazi struct_basic() -> Namba {
  weka p = Pika { x: 3, y: "hi" }
  rejesha p.x
}

kazi struct_neno() -> Neno {
  weka p = Pika { x: 1, y: "world" }
  rejesha p.y
}

kazi hamna_kama_namba() -> Namba {
  rejesha Hamna kama Namba
}

kazi orodha_with_hamna() -> Namba {
  weka a = orodha(1, 2)
  a.ongeza()
  rejesha a.urefu()
}

kazi neno_grapheme() -> Namba {
  weka s = "é"
  rejesha s.urefu()
}

kazi namba_precedence() -> Namba {
  rejesha - 1 + 10
}

kazi neno_kata_byte() -> Neno {
  weka s = "é"
  rejesha s.kata(0, 2)
}

kazi neno_e_biti_ngapi() -> Namba {
  weka s = "é"
  rejesha s.biti_ngapi()
}

kazi neno_tafuta_missing() -> Neno {
  weka s = "hello"
  weka val = s.tafuta("x")
  linganisha val {
    Hamna => { rejesha "Hamna" }
    _ => { rejesha "Some" } }
}

kazi ukweli_hamna_cast() -> Ukweli {
  rejesha Hamna kama Ukweli
}

kazi orodha_ondoa_oob() -> Neno {
  weka a = orodha(10, 20)
  weka val = a.ondoa(10)
  linganisha val {
    Hamna => { rejesha "Chaguo(None)" }
    _ => { rejesha "Some" } }
}

kazi jozi_with_hamna() -> Namba {
  weka p = jozi(Hamna, 7)
  rejesha (p.pili()) kama Namba
}

kazi herufi_zero() -> Herufi {
  rejesha 0 kama Herufi
}

kazi struct_kama_neno() -> Neno {
  weka p = Pika { x: 1, y: "a" }
  rejesha p kama Neno
}

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
