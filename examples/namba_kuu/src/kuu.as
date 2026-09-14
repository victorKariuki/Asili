leta matumizi
leta hisabati

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    // Namba_Kuu: namba kamili isiyo na kikomo -- 50! ni kubwa mno kwa Namba (f64) kubaki sahihi.
    weka faktoriali = jaribu (namba_kuu_kutoka("1"))
    weka i = jaribu (namba_kuu_kutoka("1"))
    weka mwisho = jaribu (namba_kuu_kutoka("21"))
    wakati i < mwisho {
        weka faktoriali_mpya = faktoriali * i
        faktoriali = faktoriali_mpya
        weka i_mpya = i + (jaribu (namba_kuu_kutoka("1")))
        i = i_mpya
    }
    chapisha("20! = " + (faktoriali kama Neno))

    // Namba_Sahihi: desimali sahihi -- 0.1 + 0.2 == 0.3 halisi, si 0.30000000000000004.
    weka a = jaribu (namba_sahihi_kutoka("0.1"))
    weka b = jaribu (namba_sahihi_kutoka("0.2"))
    chapisha("0.1 + 0.2 (Namba_Sahihi) = " + ((a + b) kama Neno))
    chapisha("0.1 + 0.2 (Namba ya kawaida) = " + ((0.1 + 0.2) kama Neno))
}
