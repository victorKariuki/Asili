leta matumizi

sifa Inayoonyeshwa {
    kazi onyesha(self: Self) -> Neno
}

umbo Paka { jina: Neno }
umbo Mbwa { jina: Neno, mwenye: Neno }

shughuli ya Paka kwa Inayoonyeshwa {
    kazi onyesha(self: Paka) -> Neno {
        rejesha "Paka(" + self.jina + ")"
    }
}

// Sintaksia ya nukta-mbili, sawa na "kwa" hapo juu.
shughuli ya Mbwa: Inayoonyeshwa {
    kazi onyesha(self: Mbwa) -> Neno {
        rejesha "Mbwa(" + self.jina + ", mwenye: " + self.mwenye + ")"
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka pk = Paka { jina: "Whiskers" }
    weka mb = Mbwa { jina: "Rex", mwenye: "Amara" }
    chapisha(pk.onyesha())
    chapisha(mb.onyesha())
}
