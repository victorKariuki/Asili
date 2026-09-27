thabiti N: Namba = 9
thabiti S: Namba = 81

kazi onyesha(b: Orodha<Namba>) -> Tupu {
    chapisha(b.vipande(N).ramani("mstari").jiunge("\n"))
}

kazi mstari(b: Orodha<Namba>) -> Neno {
    rejesha b.kwa_neno().jiunge(" ")
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka b: Orodha<Namba> = [8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 9.0, 0.0, 2.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 5.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 6.0, 8.0, 0.0, 0.0, 8.0, 5.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 9.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0]
    # Bit masks of the digits already used in each row, column and 3x3 box.
    weka safu: Orodha<Namba> = orodha_rudia(0, N), nguzo: Orodha<Namba> = orodha_rudia(0, N), sanduku: Orodha<Namba> = orodha_rudia(0, N)
    kwa i kutoka 0 hadi S {
        weka v = b[i]
        ikiwa v != 0 {
            weka r = i // N, c = i % N
            weka x = 1 << (v - 1)
            safu[r] |= x
            nguzo[c] |= x
            sanduku[r // 3 * 3 + c // 3] |= x
        }
    }
    # Iterative MRV backtracking: always branch on the empty cell with the fewest candidates.
    weka seli: Orodha<Namba> = [], ijayo: Orodha<Namba> = [], kina: Namba = 0, majaribio: Namba = 0, marudio: Namba = 0
    weka imekamilika: Ukweli = si_kweli
    wakati kina >= 0 {
        ikiwa kina == S {
            imekamilika = kweli vunja
        }
        ikiwa seli.urefu() == kina {
            weka bora = - 1, idadi = 10
            kwa p kutoka 0 hadi S {
                ikiwa b[p] == 0 {
                    weka r = p // N, c = p % N
                    weka tumika = safu[r] | nguzo[c] | sanduku[r // 3 * 3 + c // 3]
                    weka n: Namba = 0
                    kwa v kutoka 1 hadi 10 {
                        ikiwa tumika & (1 << (v - 1)) == 0 {
                            n += 1
                        }
                    }
                    ikiwa n < idadi {
                        bora = p idadi = n
                    }
                }
            }
            ikiwa bora == - 1 {
                imekamilika = kweli vunja
            }
            seli.ongeza(bora)
            ijayo.ongeza(1)
        }
        weka p = seli[kina]
        weka r = p // N, c = p % N
        weka q = r // 3 * 3 + c // 3
        weka tumika = safu[r] | nguzo[c] | sanduku[q]
        weka v = ijayo[kina]
        weka imewekwa: Ukweli = si_kweli
        wakati v <= N {
            majaribio += 1
            weka x = 1 << (v - 1)
            ikiwa tumika & x == 0 {
                b[p] = v
                safu[r] |= x
                nguzo[c] |= x
                sanduku[q] |= x
                ijayo[kina] = v + 1
                kina += 1
                imewekwa = kweli
                vunja
            }
            v += 1
        }
        ikiwa siyo imewekwa {
            seli.ondoa(kina)
            ijayo.ondoa(kina)
            kina -= 1
            marudio += 1
            ikiwa kina >= 0 {
                weka zamani = seli[kina]
                weka zr = zamani // N, zc = zamani % N
                weka x = 1 << (b[zamani] - 1)
                b[zamani] = 0
                safu[zr] ^= x
                nguzo[zc] ^= x
                sanduku[zr // 3 * 3 + zc // 3] ^= x
            }
        }
    }
    ikiwa imekamilika {
        onyesha(b)
        chapisha("Majaribio: " + (majaribio kama Neno))
        chapisha("Marudio: " + (marudio kama Neno))
    } vinginevyo {
        chapisha("Hakuna suluhisho.")
    }
}
