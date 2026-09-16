# Karibu Asili! Andika msimbo wako hapa kisha bofya "Endesha" (Run).
leta matumizi

kazi fibo(n: Namba) -> Namba {
  ikiwa n < 2 {
    rejesha n
  }
  rejesha fibo(n - 1) + fibo(n - 2)
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("Habari, Asili!")

  kwa i kutoka 0 hadi 9 {
    chapisha("fibo(" + (i kama Neno) + ") = " + (fibo(i) kama Neno))
  }
}
