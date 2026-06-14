leta msingi
leta matumizi

umbo Namba {
    thamani: Namba
}

shughuli ya Namba {
    kazi mara_mbili(self) -> Namba {
        rejesha self.thamani + self.thamani
    }

    kazi jumlisha(self, nyingine: Namba) -> Namba {
        rejesha self.thamani + nyingine
    }

    chapisha_self(self) -> Tupu {
        chapisha("Namba: " + namba_kuwa_neno(self.thamani))
    }
}

umbo Jozi {
    x: Namba
    y: Namba
}

shughuli ya Jozi {
    kazi jumla_kuratibu(self) -> Namba {
        rejesha self.x + self.y
    }

    kazi umbali(self) -> Namba {
        weka dx = self.x
        weka dy = self.y
        rejesha mizizi(dx * dx + dy * dy)
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("=== Phase I Module System Demo ===")
    chapisha("")

    chapisha("Umbo 1: Namba")
    weka n = Namba { thamani: 5.0 }
    n.chapisha_self()
    chapisha("  Mara mbili: " + namba_kuwa_neno(n.mara_mbili()))
    chapisha("  Jumlisha na 3: " + namba_kuwa_neno(n.jumlisha(3.0)))

    chapisha("")
    chapisha("Umbo 2: Jozi")
    weka p = Jozi { x: 3.0, y: 4.0 }
    chapisha("  Kuratibu: (" + namba_kuwa_neno(p.x) + ", " + namba_kuwa_neno(p.y) + ")")
    chapisha("  Jumla Kuratibu: " + namba_kuwa_neno(p.jumla_kuratibu()))
    chapisha("  Umbali: " + namba_kuwa_neno(p.umbali()))
}

kazi namba_kuwa_neno(n: Namba) -> Neno {
    ikiwa n == 0.0 {
        rejesha "sifuri"
    }
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
    ikiwa n == 6.0 {
        rejesha "sita"
    }
    ikiwa n == 7.0 {
        rejesha "saba"
    }
    ikiwa n == 8.0 {
        rejesha "nane"
    }
    ikiwa n == 9.0 {
        rejesha "tisa"
    }
    ikiwa n == 10.0 {
        rejesha "kumi"
    }
    ikiwa n == 5.0 {
        rejesha "tano"
    }
    rejesha "namba nyingine"
}
