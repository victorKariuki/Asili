leta faili
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka njia = "kilele/faili_mfano.txt"

    weka w = jaribu (faili_fungua(njia.clona(), "andika"))
    jaribu (w.andika("Habari kutoka Asili!\n"))
    w.funga()

    // Faili hufungwa kiotomatiki hata bila .funga() wazi -- tazama nje-ya-wigo hapa chini.
    ikiwa kweli {
        weka wa_muda = jaribu (faili_fungua(njia.clona(), "ongeza"))
        jaribu (wa_muda.andika("Mstari wa pili.\n"))
    }
    // `wa_muda` alitoka nje ya wigo pale bila .funga(); OS handle bado ilifungwa.

    weka r = jaribu (faili_fungua(njia, "soma"))
    weka maudhui = jaribu (r.soma())
    chapisha(maudhui)

    weka k = kumbukumbu_unda(42.0)
    chapisha("Kumbukumbu: " + (k.pata() kama Neno))
}
