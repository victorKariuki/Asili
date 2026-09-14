leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka wageni = seti("Amara", "Baraka", "Chidi", "Amara")
    chapisha("Wageni wa kipekee: " + (wageni.urefu() kama Neno))

    wageni.ongeza("Deka")
    chapisha("Deka yupo: " + (wageni.ina("Deka") kama Neno))

    weka aliondolewa = wageni.ondoa("Baraka")
    chapisha("Baraka aliondolewa: " + (aliondolewa kama Neno))
    chapisha("Idadi baada: " + (wageni.urefu() kama Neno))

    weka orodha_ya_wageni = wageni.orodha()
    chapisha("Kama orodha: " + (orodha_ya_wageni.urefu() kama Neno) + " washiriki")
}
