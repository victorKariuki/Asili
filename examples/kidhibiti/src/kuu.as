# Kidhibiti: a bounded control step. `dhibiti` and `wastani` are strict (`#[salama]`): the
# build checks they keep to the bounded subset and prints the most steps and memory a call can
# use. The list of samples is made once, by ordinary code, and passed in.


thabiti N: Namba = 8

#[salama]
kazi wastani(sampuli: Orodha<Namba>) -> Namba {
    weka jumla = 0
    kwa i kutoka 0 hadi N {
        jumla += sampuli[i]
    }
    rejesha jumla / N
}

#[salama]
kazi dhibiti(sampuli: Orodha<Namba>, lengo: Namba) -> Namba {
    weka kosa = lengo - wastani(sampuli)
    ikiwa kosa > 10 {
        rejesha 10
    } au_ikiwa kosa < -10 {
        rejesha -10
    }
    rejesha kosa
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka s = [0; 8]
    kwa i kutoka 0 hadi 8 {
        s[i] = i
    }
    chapisha(dhibiti(s, 5) kama Neno)
}
