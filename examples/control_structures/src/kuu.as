leta mfumo
leta matumizi

kazi demo_ikiwa() -> Namba {
  ikiwa kweli { rejesha 1 }
  rejesha 0
}

kazi demo_ikiwa_vinginevyo() -> Namba {
  weka x = 2
  ikiwa x < 0 { rejesha -1 }
  vinginevyo { rejesha 1 }
  rejesha 0
}

kazi demo_au_ikiwa() -> Namba {
  weka n = 2
  ikiwa n == 0 { rejesha 0 }
  au_ikiwa n == 1 { rejesha 1 }
  au_ikiwa n == 2 { rejesha 2 }
  vinginevyo { rejesha -1 }
  rejesha 0
}

kazi demo_kwa_katika() -> Namba {
  weka sum = 0
  kwa x katika orodha(10, 20, 30) {
    sum = sum + x
  }
  rejesha sum
}

kazi demo_kwa_kutoka_hadi() -> Namba {
  weka sum = 0
  kwa i kutoka 0 hadi 4 {
    sum = sum + i
  }
  rejesha sum
}

kazi demo_kwa_range_empty() -> Namba {
  weka count = 0
  kwa i kutoka 0 hadi 0 {
    count = count + 1
  }
  rejesha count
}

kazi demo_wakati() -> Namba {
  weka n = 0
  wakati n < 5 {
    n = n + 1
  }
  rejesha n
}

kazi demo_wakati_milele_vunja() -> Namba {
  weka n = 0
  wakati milele {
    n = n + 1
    ikiwa n >= 3 { vunja }
  }
  rejesha n
}

kazi demo_lebo_vunja() -> Namba {
  weka n = 0
  lebo 'nje: wakati milele {
    n = n + 1
    ikiwa n >= 2 { vunja 'nje }
  }
  rejesha n
}

kazi demo_endelea() -> Namba {
  weka sum = 0
  kwa i kutoka 0 hadi 6 {
    ikiwa i == 3 { endelea }
    sum = sum + i
  }
  rejesha sum
}

kazi demo_linganisha_namba() -> Namba {
  weka x = 2
  linganisha x {
    0 => { rejesha 0 }
    1 => { rejesha 1 }
    2 => { rejesha 2 }
    _ => { rejesha -1 }
  }
  rejesha 0
}

kazi demo_linganisha_hamna() -> Namba {
  weka val = Hamna
  linganisha val {
    Hamna => { rejesha 1 }
    _ => { rejesha 0 }
  }
  rejesha 0
}

kazi demo_linganisha_jozi() -> Namba {
  weka p = jozi(3, 4)
  weka a = (p.kwanza() kama Namba)
  weka b = (p.pili() kama Namba)
  linganisha 1 {
    1 => { rejesha a + b }
    _ => { rejesha 0 }
  }
  rejesha 0
}

kazi demo_thabiti() -> Namba {
  thabiti x = 7
  rejesha x
}

kazi demo_tupa() -> Namba {
  weka z = 10
  tupa z
  rejesha 0
}

kazi demo_propagate_ok() -> Namba {
  weka a = orodha(5, 15, 25)
  rejesha a[1]?
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- Control structures ---")
  chapisha("ikiwa: " + (demo_ikiwa() kama Neno))
  chapisha("ikiwa + vinginevyo: " + (demo_ikiwa_vinginevyo() kama Neno))
  chapisha("ikiwa + au_ikiwa + vinginevyo: " + (demo_au_ikiwa() kama Neno))
  chapisha("kwa katika: sum = " + (demo_kwa_katika() kama Neno))
  chapisha("kwa kutoka 0 hadi 4: sum = " + (demo_kwa_kutoka_hadi() kama Neno))
  chapisha("kwa 0 hadi 0 (empty): count = " + (demo_kwa_range_empty() kama Neno))
  chapisha("wakati n<5: n = " + (demo_wakati() kama Neno))
  chapisha("wakati milele + vunja: n = " + (demo_wakati_milele_vunja() kama Neno))
  chapisha("lebo + vunja label: n = " + (demo_lebo_vunja() kama Neno))
  chapisha("endelea (skip 3): sum = " + (demo_endelea() kama Neno))
  chapisha("linganisha namba 2: " + (demo_linganisha_namba() kama Neno))
  chapisha("linganisha Hamna: " + (demo_linganisha_hamna() kama Neno))
  chapisha("linganisha jozi (3,4): " + (demo_linganisha_jozi() kama Neno))
  chapisha("thabiti x=7: " + (demo_thabiti() kama Neno))
  chapisha("tupa z then 0: " + (demo_tupa() kama Neno))
  chapisha("a[1]? on orodha(5,15,25): " + (demo_propagate_ok() kama Neno))
  chapisha("--- mwisho ---")
}
