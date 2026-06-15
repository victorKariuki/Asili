leta matumizi

# This file demonstrates the enhanced syntax highlighting

#[sharti(os = "linux", toleo = 1)]
kazi jumla(x: Namba, y: Namba) -> Namba {
  # Function parameters (x, y) are visually distinct from regular variables
  weka ufumbuzi = x + y
  rejesha ufumbuzi
}

kazi hesabu_nguvu(msingi: Namba, kielelezo: Namba) -> Namba {
  weka matokeo = msingi ** kielelezo
  rejesha matokeo
}

umbo Mtu {
  jina: Neno
  umri: Biti32
  ni_hai: Ukweli
}

kazi jina_kamili(mtu: Mtu) -> Neno {
  # Member access: mtu.jina gets variable.other.member scope
  weka sehemu_a: Neno = mtu.jina
  rejesha sehemu_a
}

# Operators are semantically grouped:
# - Arithmetic: + - * / %
# - Comparison: == != < > <= >=
# - Logical: && ||
# - Bitwise: na_biti au_biti xor_biti

kazi onyesha_waendeshaji() -> Tupu {
  weka idadi = 10

  # Arithmetic operators
  weka jumla = 5 + 3
  weka tofauti = 10 - 4
  weka bidhaa = 6 * 7
  weka mgawanyiko = 20 / 4

  # Comparison operators
  ikiwa idadi == 10 {
    chapisha("Sawa")
  }

  ikiwa idadi != 5 {
    chapisha("Tofauti")
  }

  # Logical operators
  ikiwa kweli na kweli {
    chapisha("Mantiki")
  }

  # Bitwise operators
  weka bits = 12 na_biti 8
}

# Control flow keywords
kazi mzunguko_rahisi() -> Tupu {
  wakati kweli {
    chapisha("Kurudia")
    vunja
  }

  kwa i kutoka 0 hadi 10 {
    ikiwa i == 5 {
      endelea
    }
    chapisha(i kama Neno)
  }
}

# Attributes
#[jaribio]
kazi mtihani_huru() -> Ukweli {
  rejesha kweli
}

# Lists
kazi nguvu_zote() -> Tupu {
  weka orodha = [1, 2, 3, 4, 5]

  kwa kipengee katika orodha {
    chapisha(kipengee kama Neno)
  }
}
