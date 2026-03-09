leta mfumo
leta hisabati
leta matumizi

kazi heuristiki(gharama_id: Namba, lengo_id: Namba, cols: Namba) -> Namba {
  weka r1 = sakafu(gharama_id / cols)
  weka c1 = gharama_id - r1 * cols
  weka r2 = sakafu(lengo_id / cols)
  weka c2 = lengo_id - r2 * cols
  rejesha absolute(r1 - r2) + absolute(c1 - c2)
}

kazi ndani_ya_mpaka(id: Namba, cols: Namba, rows: Namba) -> Ukweli {
  ikiwa id < 0 { rejesha si_kweli }
  ikiwa id >= cols * rows { rejesha si_kweli }
  rejesha kweli
}

kazi zimezunguka(vizuizi: Orodha<Namba>, id: Namba) -> Ukweli {
  kwa v katika vizuizi {
    ikiwa v == id { rejesha kweli }
  }
  rejesha si_kweli
}

kazi jirani(id: Namba, cols: Namba, rows: Namba, vizuizi: Orodha<Namba>) -> Orodha<Namba> {
  weka r = sakafu(id / cols)
  weka c = id - r * cols
  weka j = orodha()
  weka juu = id - cols
  weka chini = id + cols
  ikiwa ndani_ya_mpaka(juu, cols, rows) na siyo zimezunguka(vizuizi, juu) { j.ongeza(juu) }
  ikiwa ndani_ya_mpaka(chini, cols, rows) na siyo zimezunguka(vizuizi, chini) { j.ongeza(chini) }
  ikiwa c > 0 {
    weka kushoto = id - 1
    ikiwa siyo zimezunguka(vizuizi, kushoto) { j.ongeza(kushoto) }
  }
  ikiwa c < cols - 1 {
    weka kulia = id + 1
    ikiwa siyo zimezunguka(vizuizi, kulia) { j.ongeza(kulia) }
  }
  rejesha j
}

kazi astar(mwanzo: Namba, lengo: Namba, cols: Namba, rows: Namba, vizuizi: Orodha<Namba>) -> Orodha<Namba> {
  weka open = orodha(mwanzo)
  weka g = kamusi_tupu()
  weka came_from = kamusi_tupu()
  weka closed = kamusi_tupu()
  g.ingiza(mwanzo, 0)
  came_from.ingiza(mwanzo, 0 - 1)
  wakati milele {
    ikiwa open.urefu() == 0 { rejesha orodha() }

    weka bora_f = 999999
    weka bora_id = 0 - 1
    kwa id katika open {
      linganisha closed.pata(id) {
        Hamna => {
          weka gv = (g.pata(id).angu(999999) kama Namba)
          weka h = heuristiki(id, lengo, cols)
          weka f = gv + h
          ikiwa f < bora_f {
            bora_f = f
            bora_id = id
          }
        }
        _ => {}
      }
    }

    ikiwa bora_id < 0 { rejesha orodha() }
    weka n = bora_id

    ikiwa n == lengo {
      weka path = orodha()
      weka current = lengo
      wakati milele {
        path.ongeza(current)
        weka parent = (came_from.pata(current).angu(0 - 1) kama Namba)
        ikiwa parent < 0 { vunja }
        weka current = parent
      }
      rejesha path
    }

    closed.ingiza(n, kweli)

    weka new_open = orodha()
    kwa k katika open {
      ikiwa k != n { new_open.ongeza(k) }
    }

    weka g_val = (g.pata(n).angu(999999) kama Namba)
    kwa j katika jirani(n, cols, rows, vizuizi) {
      linganisha closed.pata(j) {
        Hamna => {
          weka tent_g = g_val + 1
          weka prev = (g.pata(j).angu(999999) kama Namba)
          ikiwa tent_g < prev {
            g.ingiza(j, tent_g)
            came_from.ingiza(j, n)
            weka tayari_imo = si_kweli
            kwa k katika new_open {
              ikiwa k == j { tayari_imo = kweli }
            }
            ikiwa siyo tayari_imo { new_open.ongeza(j) }
          }
        }
        _ => {}
      }
    }

    open = new_open
  }
}

kazi jaribio_heuristiki() -> Tupu {
  chapisha("--- Jaribio: heuristiki ---")
  weka cols = 5
  chapisha("heuristiki(0, 24, 5) = " + (heuristiki(0, 24, 5) kama Neno))
  chapisha("heuristiki(0, 0, 5) = " + (heuristiki(0, 0, 5) kama Neno))
  chapisha("heuristiki(12, 24, 5) = " + (heuristiki(12, 24, 5) kama Neno))
}

kazi jaribio_ndani_ya_mpaka() -> Tupu {
  chapisha("--- Jaribio: ndani_ya_mpaka ---")
  weka cols = 5
  weka rows = 5
  chapisha("ndani_ya_mpaka(-1,5,5) = " + (ndani_ya_mpaka(0 - 1, cols, rows) kama Neno))
  chapisha("ndani_ya_mpaka(25,5,5) = " + (ndani_ya_mpaka(25, cols, rows) kama Neno))
  chapisha("ndani_ya_mpaka(0,5,5) = " + (ndani_ya_mpaka(0, cols, rows) kama Neno))
  chapisha("ndani_ya_mpaka(24,5,5) = " + (ndani_ya_mpaka(24, cols, rows) kama Neno))
}

kazi jaribio_zimezunguka() -> Tupu {
  chapisha("--- Jaribio: zimezunguka ---")
  weka vizuizi = orodha(6, 7, 8)
  chapisha("zimezunguka(vizuizi, 7) = " + (zimezunguka(vizuizi, 7) kama Neno))
  chapisha("zimezunguka(vizuizi, 0) = " + (zimezunguka(vizuizi, 0) kama Neno))
}

kazi jaribio_jirani() -> Tupu {
  chapisha("--- Jaribio: jirani ---")
  weka cols = 5
  weka rows = 5
  weka vizuizi_tupu = orodha()
  weka j0 = jirani(0, cols, rows, vizuizi_tupu)
  chapisha("jirani(0) urefu: " + (j0.urefu() kama Neno))
  chapisha("jirani(0) ids:")
  kwa x katika j0 {
    chapisha("  " + (x kama Neno))
  }
  weka j4 = jirani(4, cols, rows, vizuizi_tupu)
  chapisha("jirani(4) urefu: " + (j4.urefu() kama Neno))
  chapisha("jirani(4) ids:")
  kwa x katika j4 {
    chapisha("  " + (x kama Neno))
  }
  weka vizuizi = orodha(6, 7, 8)
  weka j5 = jirani(5, cols, rows, vizuizi)
  chapisha("jirani(5) na vizuizi urefu: " + (j5.urefu() kama Neno))
  chapisha("jirani(5) ids:")
  kwa x katika j5 {
    chapisha("  " + (x kama Neno))
  }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  jaribio_heuristiki()
  jaribio_ndani_ya_mpaka()
  jaribio_zimezunguka()
  jaribio_jirani()
  chapisha("--- A* kamili ---")
  weka cols = 5
  weka rows = 5
  weka mwanzo = 0
  weka lengo = 24
  weka vizuizi = orodha(6, 7, 8, 12, 17)
  weka path = astar(mwanzo, lengo, cols, rows, vizuizi)
  chapisha("A* path urefu: " + (path.urefu() kama Neno))
  kwa id katika path {
    chapisha("  id: " + (id kama Neno))
  }
}
