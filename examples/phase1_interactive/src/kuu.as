leta msingi
leta matumizi
leta majira

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("=== Phase I Interactive Demo ===")
    chapisha("")

    chapisha("Ingiza jina lako: ")
    weka jina = omba()
    chapisha("Habari, " + jina + "!")

    chapisha("")
    chapisha("Wakati wa sasa: " + umbiza(majira()))

    chapisha("")
    chapisha("Sifa za mfumo:")
    chapisha("  Architecture: " + arch())
    chapisha("  Is Debug Build: " + kweli_au_siyo(ni_debug()))
    chapisha("  Is WebAssembly: " + kweli_au_siyo(ni_wasm()))

    chapisha("")
    chapisha("Orodha: 1, 2, 3, 4, 5")
    weka orodha = orodha(1.0, 2.0, 3.0, 4.0, 5.0)
    orodha.kila_mmoja("chapisha_namba")
}

kazi kweli_au_siyo(k: Ukweli) -> Neno {
    ikiwa k {
        rejesha "Ndiyo"
    }
    rejesha "Hapana"
}

kazi chapisha_namba(n: Namba) -> Tupu {
    chapisha("  - " + namba_kuwa_neno(n))
}

kazi namba_kuwa_neno(n: Namba) -> Neno {
    ikiwa n == 1.0 {
        rejesha "moja"
    }
    ikiwa n == 2.0 {
        rejesha "mbili"
    }
    ikiwa n == 3.0 {
        rejesha "tatu"
    }
    ikiwa n == 4.0 {
        rejesha "nne"
    }
    ikiwa n == 5.0 {
        rejesha "tano"
    }
    rejesha "nyingi"
}
