leta hisabati
leta mfumo
leta matumizi

kazi neg_double() -> Namba {
  rejesha --1
}

kazi neg_zero() -> Namba {
  rejesha -0
}

kazi neg_ukomo() -> Namba {
  rejesha -Ukomo
}

kazi neg_siyo_namba() -> Namba {
  rejesha -Siyo_Namba
}

kazi neg_precedence() -> Namba {
  rejesha -1 + 10
}

kazi not_double() -> Ukweli {
  rejesha siyo siyo kweli
}

kazi not_triple() -> Ukweli {
  rejesha siyo siyo siyo kweli
}

kazi bitnot_zero() -> Namba {
  rejesha siyo_biti 0
}

kazi bitnot_neg_one() -> Namba {
  rejesha siyo_biti (0-1)
}

kazi bitnot_three() -> Namba {
  rejesha siyo_biti 3
}

kazi jaribu_tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

kazi jaribu_chaguo_some() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

kazi precedence_binary_minus() -> Namba {
  rejesha 0-1
}

kazi precedence_unary_neg() -> Namba {
  rejesha -1
}

kazi literal_neg_one() -> Namba {
  rejesha -1
}

kazi unary_neg_one() -> Namba {
  rejesha - 1
}

kazi binary_minus_spaced() -> Namba {
  rejesha 0-1
}

kazi binary_minus_no_space() -> Namba {
  rejesha 0-1
}

kazi neg_triple() -> Namba {
  rejesha ---1
}

kazi neg_before_mul() -> Namba {
  rejesha -2*3
}

kazi not_quad() -> Ukweli {
  rejesha siyo siyo siyo siyo kweli
}

kazi bitnot_expr() -> Namba {
  rejesha siyo_biti (1+2)
}

kazi neg_of_bitnot_zero() -> Namba {
  rejesha - siyo_biti 0
}

kazi neg_siyo_biti_no_space() -> Namba {
  rejesha -siyo_biti 0
}

kazi neg_siyo_biti_spaced() -> Namba {
  rejesha - siyo_biti 0
}

kazi binary_minus_unary_neg() -> Namba {
  rejesha 10 - - 1
}

kazi not_of_comparison() -> Ukweli {
  rejesha siyo (0 == 1)
}

kazi neg_cast_to_neno() -> Neno {
  rejesha (- 7) kama Neno
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- Unary ops ---")
  chapisha("- - 1 = " + (neg_double() kama Neno))
  chapisha("- 0 = " + (neg_zero() kama Neno))
  chapisha("- Ukomo = " + (neg_ukomo() kama Neno))
  chapisha("- Siyo_Namba = " + (neg_siyo_namba() kama Neno))
  chapisha("- 1 + 10 = " + (neg_precedence() kama Neno))
  chapisha("siyo siyo kweli = " + (not_double() kama Neno))
  chapisha("siyo siyo siyo kweli = " + (not_triple() kama Neno))
  chapisha("siyo_biti 0 = " + (bitnot_zero() kama Neno))
  chapisha("siyo_biti (0-1) = " + (bitnot_neg_one() kama Neno))
  chapisha("siyo_biti 3 = " + (bitnot_three() kama Neno))
  chapisha("jaribu gawio(10,2) = " + (jaribu_tokeo_ok() kama Neno))
  chapisha("jaribu (100 kama Biti8) = " + (jaribu_chaguo_some() kama Neno))
  chapisha("0 - 1 = " + (precedence_binary_minus() kama Neno))
  chapisha("- 1 = " + (precedence_unary_neg() kama Neno))
  chapisha("--- -1 vs - 1 (unary neg: same) ---")
  chapisha("-1 = " + (literal_neg_one() kama Neno))
  chapisha("- 1 = " + (unary_neg_one() kama Neno))
  chapisha("(-1 == - 1) = " + ((literal_neg_one() == unary_neg_one()) kama Neno))
  chapisha("--- 0 - 1 vs 0-1 (binary minus: same) ---")
  chapisha("0 - 1 = " + (binary_minus_spaced() kama Neno))
  chapisha("0-1 = " + (binary_minus_no_space() kama Neno))
  chapisha("(0 - 1 == 0-1) = " + ((binary_minus_spaced() == binary_minus_no_space()) kama Neno))
  chapisha("--- Push further ---")
  chapisha("- - - 1 = " + (neg_triple() kama Neno))
  chapisha("- 2 * 3 = " + (neg_before_mul() kama Neno))
  chapisha("siyo^4 kweli = " + (not_quad() kama Neno))
  chapisha("siyo_biti (1+2) = " + (bitnot_expr() kama Neno))
  chapisha("- siyo_biti 0 = " + (neg_of_bitnot_zero() kama Neno))
  chapisha("--- -siyo_biti vs - siyo_biti (same) ---")
  chapisha("-siyo_biti 0 = " + (neg_siyo_biti_no_space() kama Neno))
  chapisha("- siyo_biti 0 = " + (neg_siyo_biti_spaced() kama Neno))
  chapisha("(-siyo_biti 0 == - siyo_biti 0) = " + ((neg_siyo_biti_no_space() == neg_siyo_biti_spaced()) kama Neno))
  chapisha("10 - - 1 = " + (binary_minus_unary_neg() kama Neno))
  chapisha("siyo (0==1) = " + (not_of_comparison() kama Neno))
  chapisha("(- 7) kama Neno = " + neg_cast_to_neno())
  chapisha("--- mwisho ---")
}
