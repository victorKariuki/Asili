leta hisabati
leta mfumo
leta matumizi

thabiti LABEL_NEG_ONE: Neno = "- 1 = "
thabiti LABEL_NEG_BITNOT_ZERO: Neno = "- siyo_biti 0 = "
thabiti LABEL_ZERO_MINUS_ONE: Neno = "0 - 1 = "

# Double negation: -(-1).
kazi neg_double() -> Namba {
  rejesha --1
}

# Negation of zero.
kazi neg_zero() -> Namba {
  rejesha -0
}

# Negation of the Namba max-value constant.
kazi neg_ukomo() -> Namba {
  rejesha -Ukomo
}

# Negation of the not-a-number constant.
kazi neg_siyo_namba() -> Namba {
  rejesha -Siyo_Namba
}

# Unary negation binds tighter than addition.
kazi neg_precedence() -> Namba {
  rejesha -1 + 10
}

# Double logical negation cancels out.
kazi not_double() -> Ukweli {
  rejesha siyo siyo kweli
}

# Triple logical negation.
kazi not_triple() -> Ukweli {
  rejesha siyo siyo siyo kweli
}

# Bitwise NOT of zero.
kazi bitnot_zero() -> Namba {
  rejesha siyo_biti 0
}

# Bitwise NOT of a negative number.
kazi bitnot_neg_one() -> Namba {
  rejesha siyo_biti (0-1)
}

# Bitwise NOT of a positive number.
kazi bitnot_three() -> Namba {
  rejesha siyo_biti 3
}

# `jaribu` unwrapping a successful Tokeo.
kazi jaribu_tokeo_ok() -> Namba {
  rejesha jaribu gawio(10, 2)
}

# `jaribu` unwrapping a successful cast (Chaguo).
kazi jaribu_chaguo_some() -> Namba {
  rejesha jaribu (100 kama Biti8)
}

# Binary minus without spaces around the operator.
kazi precedence_binary_minus() -> Namba {
  rejesha 0-1
}

# Unary negation on a literal.
kazi precedence_unary_neg() -> Namba {
  rejesha -1
}

# `-1` written as a single literal token.
kazi literal_neg_one() -> Namba {
  rejesha -1
}

# `-1` written as unary negation with a space.
kazi unary_neg_one() -> Namba {
  rejesha - 1
}

# `0-1` written with a leading space before the operator.
kazi binary_minus_spaced() -> Namba {
  rejesha 0-1
}

# `0-1` written with no spaces at all.
kazi binary_minus_no_space() -> Namba {
  rejesha 0-1
}

# Triple negation: -(-(-1)).
kazi neg_triple() -> Namba {
  rejesha ---1
}

# Unary negation binds tighter than multiplication.
kazi neg_before_mul() -> Namba {
  rejesha -2*3
}

# Four chained logical negations.
kazi not_quad() -> Ukweli {
  rejesha siyo siyo siyo siyo kweli
}

# Bitwise NOT of a parenthesized addition.
kazi bitnot_expr() -> Namba {
  rejesha siyo_biti (1+2)
}

# Unary negation applied to a bitwise-NOT result.
kazi neg_of_bitnot_zero() -> Namba {
  rejesha - siyo_biti 0
}

# `-siyo_biti 0` written with no space before `siyo_biti`.
kazi neg_siyo_biti_no_space() -> Namba {
  rejesha -siyo_biti 0
}

# `- siyo_biti 0` written with a space before `siyo_biti`.
kazi neg_siyo_biti_spaced() -> Namba {
  rejesha - siyo_biti 0
}

# Binary minus followed by unary negation: `10 - (-1)`.
kazi binary_minus_unary_neg() -> Namba {
  rejesha 10 - - 1
}

# Logical NOT of a comparison expression.
kazi not_of_comparison() -> Ukweli {
  rejesha siyo (0 == 1)
}

# Negation result cast to a string.
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
  chapisha(LABEL_ZERO_MINUS_ONE + (precedence_binary_minus() kama Neno))
  chapisha(LABEL_NEG_ONE + (precedence_unary_neg() kama Neno))
  chapisha("--- -1 vs - 1 (unary neg: same) ---")
  chapisha("-1 = " + (literal_neg_one() kama Neno))
  chapisha(LABEL_NEG_ONE + (unary_neg_one() kama Neno))
  chapisha("(-1 == - 1) = " + ((literal_neg_one() == unary_neg_one()) kama Neno))
  chapisha("--- 0 - 1 vs 0-1 (binary minus: same) ---")
  chapisha(LABEL_ZERO_MINUS_ONE + (binary_minus_spaced() kama Neno))
  chapisha("0-1 = " + (binary_minus_no_space() kama Neno))
  chapisha("(0 - 1 == 0-1) = " + ((binary_minus_spaced() == binary_minus_no_space()) kama Neno))
  chapisha("--- Push further ---")
  chapisha("- - - 1 = " + (neg_triple() kama Neno))
  chapisha("- 2 * 3 = " + (neg_before_mul() kama Neno))
  chapisha("siyo^4 kweli = " + (not_quad() kama Neno))
  chapisha("siyo_biti (1+2) = " + (bitnot_expr() kama Neno))
  chapisha(LABEL_NEG_BITNOT_ZERO + (neg_of_bitnot_zero() kama Neno))
  chapisha("--- -siyo_biti vs - siyo_biti (same) ---")
  chapisha("-siyo_biti 0 = " + (neg_siyo_biti_no_space() kama Neno))
  chapisha(LABEL_NEG_BITNOT_ZERO + (neg_siyo_biti_spaced() kama Neno))
  chapisha("(-siyo_biti 0 == - siyo_biti 0) = " + ((neg_siyo_biti_no_space() == neg_siyo_biti_spaced()) kama Neno))
  chapisha("10 - - 1 = " + (binary_minus_unary_neg() kama Neno))
  chapisha("siyo (0==1) = " + (not_of_comparison() kama Neno))
  chapisha("(- 7) kama Neno = " + neg_cast_to_neno())
  chapisha("--- mwisho ---")
}
