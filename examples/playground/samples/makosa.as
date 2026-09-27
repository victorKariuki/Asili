leta matumizi

kazi gawanya(a: Namba, b: Namba) -> Tokeo<Namba, Neno> {
  ikiwa b == 0 {
    rejesha kosa("Haiwezekani kugawanya na sifuri")
  }
  rejesha tokeo(a / b)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  linganisha gawanya(10, 2) {
    Tokeo::Sawa(v)  => { chapisha("Jibu: " + (v kama Neno)) }
    Tokeo::Kosa(e) => { chapisha("Kosa: " + e) }
  }
  linganisha gawanya(10, 0) {
    Tokeo::Sawa(v)  => { chapisha("Jibu: " + (v kama Neno)) }
    Tokeo::Kosa(e) => { chapisha("Kosa: " + e) }
  }
}
