thabiti N: Namba = 9
thabiti S: Namba = 81

kazi onyesha(b: Orodha<Namba>) -> Tupu {
    chapisha(b.vipande(N).ramani("mstari").jiunge("\n"))
}

kazi mstari(b: Orodha<Namba>) -> Neno {
    rejesha b.kwa_neno().jiunge(" ")
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka b: Orodha<Namba> = [8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 9.0, 0.0, 2.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 5.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 6.0, 8.0, 0.0, 0.0, 8.0, 5.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0]
    weka safu_biti: Orodha<Namba> = orodha_rudia(0, 9), nguzo_biti: Orodha<Namba> = orodha_rudia(0, 9), kisanduku_biti: Orodha<Namba> = orodha_rudia(0, 9)
    kwa i kutoka 0 hadi S {
        weka v = b[i]?
        ikiwa v != 0 {
            weka r = sakafu(i / N)
            weka c = i % N
            weka q = sakafu(r / 3) * 3 + sakafu(c / 3)
            weka x = 1 << (v - 1)
            safu_biti[r] = safu_biti[r]? | x
            nguzo_biti[c] = nguzo_biti[c]? | x
            kisanduku_biti[q] = kisanduku_biti[q]? | x
        }
    }
    weka cells: Orodha<Namba> = [], next: Orodha<Namba> = [], depth: Namba = 0, tries: Namba = 0, backs: Namba = 0
    weka done: Ukweli = si_kweli
    wakati depth >= 0 {
        ikiwa depth == S {
            done = kweli vunja
        }
        ikiwa cells.urefu() == depth {
            weka best = - 1, count = 10
            kwa p kutoka 0 hadi S {
                ikiwa b[p]? == 0 {
                    weka r = sakafu(p / N)
                    weka c = p % N
                    weka q = sakafu(r / 3) * 3 + sakafu(c / 3)
                    weka used = safu_biti[r]? | nguzo_biti[c]? | kisanduku_biti[q]?
                    weka n: Namba = 0, v: Namba = 1
                    kwa v kutoka 1 hadi 10 {
                        weka x = 1 << (v - 1)
                        ikiwa (used & x) == 0 {
                            n += 1
                        }
                    }
                    ikiwa n < count {
                        best = p count = n
                    }
                }
            }
            ikiwa best == - 1 {
                done = kweli vunja
            }
            cells.ongeza(best)
            next.ongeza(1)
        }
        weka slot = depth
        weka p = cells[slot]?
        weka r = sakafu(p / N)
        weka c = p % N
        weka q = sakafu(r / 3) * 3 + sakafu(c / 3)
        weka used = safu_biti[r]? | nguzo_biti[c]? | kisanduku_biti[q]?
        weka v = next[slot]?
        weka found: Ukweli = si_kweli
        wakati v <= N {
            tries += 1
            weka x = 1 << (v - 1)
            ikiwa (used & x) == 0 {
                b[p] = v
                safu_biti[r] = safu_biti[r]? | x
                nguzo_biti[c] = nguzo_biti[c]? | x
                kisanduku_biti[q] = kisanduku_biti[q]? | x
                next[slot] = v + 1
                depth += 1
                found = kweli
                vunja
            }
            v += 1
        }
        ikiwa siyo found {
            cells.ondoa(slot)
            next.ondoa(slot)
            depth -= 1
            backs += 1
            ikiwa depth >= 0 {
                weka old = cells[depth]?
                weka or = sakafu(old / N)
                weka oc = old % N
                weka oq = sakafu(or / 3) * 3 + sakafu(oc / 3)
                weka x = 1 << (b[old]? - 1)
                b[old] = 0
                safu_biti[or] = safu_biti[or]? ^ x
                nguzo_biti[oc] = nguzo_biti[oc]? ^ x
                kisanduku_biti[oq] = kisanduku_biti[oq]? ^ x
            }
        }
    }
    ikiwa done {
        onyesha(b)
        chapisha("Majaribio: " + (tries kama Neno))
        chapisha("Marudio: " + (backs kama Neno))
    } vinginevyo {
        chapisha("Hakuna suluhisho.")
    }
}
