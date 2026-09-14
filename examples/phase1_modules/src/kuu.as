leta msingi
leta matumizi

# A boxed number with methods (self: Kisanduku) — kept as a struct (not named `Namba`,
# to avoid shadowing the primitive type) to demonstrate `shughuli ya`.
umbo Kisanduku {
    thamani: Namba
}

shughuli ya Kisanduku {
    # Doubles the boxed value.
    kazi mara_mbili(self: Kisanduku) -> Namba {
        rejesha self.thamani + self.thamani
    }

    # Adds another number to the boxed value.
    kazi jumlisha(self: Kisanduku, nyingine: Namba) -> Namba {
        rejesha self.thamani + nyingine
    }

    # Prints the boxed value.
    kazi chapisha_self(self: Kisanduku) -> Tupu {
        chapisha("Namba: " + namba_kuwa_neno(self.thamani))
    }
}

# A 2D coordinate pair.
umbo Jozi {
    x: Namba
    y: Namba
}

shughuli ya Jozi {
    # Sum of the two coordinates.
    kazi jumla_kuratibu(self: Jozi) -> Namba {
        rejesha self.x + self.y
    }

    # Euclidean distance from the origin.
    kazi umbali(self: Jozi) -> Namba {
        weka dx = self.x
        weka dy = self.y
        rejesha jaribu mizizi(dx * dx + dy * dy)
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    chapisha("=== Phase I Module System Demo ===")
    chapisha("")

    chapisha("Umbo 1: Kisanduku")
    weka n = Kisanduku { thamani: 5.0 }
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

# Spells out a small whole number (0-10) in Swahili; anything else returns a fallback.
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
    rejesha "namba nyingine"
}
