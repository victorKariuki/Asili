leta sambamba
leta matumizi
leta kasha_gc

kazi mfanyakazi(tx: NjiaTx<Namba>, n: Namba) -> Tupu {
    weka jumla = n * n
    jaribu (tx.tuma(jumla))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    // tenda + njia: spawn a real OS thread, get its result back over a channel.
    weka p = njia()
    weka tx = p.kwanza()
    weka rx = p.pili()

    weka id = jaribu (tenda("mfanyakazi", tx, 7))
    weka jibu = jaribu (rx.pokea())
    chapisha("7*7 kutoka kwa uzi: " + (jibu kama Neno))
    jaribu (subiri_tenda(id))

    // fungo: a mutex-protected shared value.
    weka hesabu = jaribu (fungo(0.0))
    hesabu.weka(100.0)
    chapisha("fungo: " + (hesabu.pata() kama Neno))

    // Explicit funga/fungua for holding the lock across two operations.
    hesabu.funga()
    hesabu.fungua()

    // Kasha_GC<T>/Faili/Mkondo cannot cross into a tenda-spawned thread.
    weka g = kasha_gc_unda(1.0)
    linganisha tenda("mfanyakazi", g, 1) {
        Tokeo::Sawa(_) => { chapisha("hii haipaswi kutokea") }
        Tokeo::Kosa(ujumbe) => { chapisha("kama ilivyotarajiwa: " + ujumbe) }
    }
}
