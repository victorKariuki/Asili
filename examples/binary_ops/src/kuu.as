leta mfumo
leta matumizi

kazi add_namba() -> Namba {
  rejesha 10 + 20
}

kazi add_neno() -> Neno {
  rejesha "hello" + " " + "world"
}

kazi sub_namba() -> Namba {
  rejesha 100 - 30
}

kazi mul_namba() -> Namba {
  rejesha 6 * 7
}

kazi div_namba() -> Namba {
  rejesha 15 / 4
}

kazi rem_namba() -> Namba {
  rejesha 17 % 5
}

kazi pow_namba() -> Namba {
  rejesha 2 ** 10
}

kazi precedence_mul_before_add() -> Namba {
  rejesha 1 + 2 * 3
}

kazi precedence_add_before_shift() -> Namba {
  rejesha 1 + 2 sogeza_kushoto 2
}

kazi precedence_pow_highest() -> Namba {
  rejesha 2 * 3 ** 2
}

kazi eq_namba() -> Ukweli {
  rejesha 5 == 5
}

kazi eq_neno() -> Ukweli {
  rejesha "a" == "a"
}

kazi ne_namba() -> Ukweli {
  rejesha 3 != 4
}

kazi ne_neno() -> Ukweli {
  rejesha "x" != "y"
}

kazi gt_namba() -> Ukweli {
  rejesha 10 > 5
}

kazi lt_namba() -> Ukweli {
  rejesha 3 < 7
}

kazi ge_namba() -> Ukweli {
  rejesha 5 >= 5
}

kazi le_namba() -> Ukweli {
  rejesha 4 <= 4
}

kazi gt_neno() -> Ukweli {
  rejesha "b" > "a"
}

kazi lt_neno() -> Ukweli {
  rejesha "a" < "b"
}

kazi ge_neno() -> Ukweli {
  rejesha "x" >= "x"
}

kazi le_neno() -> Ukweli {
  rejesha "x" <= "x"
}

kazi eq_hamna() -> Ukweli {
  rejesha Hamna == Hamna
}

kazi pow_zero_exp() -> Namba {
  rejesha 5 ** 0
}

kazi and_both_true() -> Ukweli {
  rejesha kweli na kweli
}

kazi and_one_false() -> Ukweli {
  rejesha kweli na si_kweli
}

kazi or_both_false() -> Ukweli {
  rejesha si_kweli au si_kweli
}

kazi or_one_true() -> Ukweli {
  rejesha kweli au si_kweli
}

kazi na_biti() -> Namba {
  rejesha 3 na_biti 5
}

kazi au_biti() -> Namba {
  rejesha 3 au_biti 5
}

kazi xor_biti() -> Namba {
  rejesha 3 xor_biti 5
}

kazi sogeza_kushoto() -> Namba {
  rejesha 1 sogeza_kushoto 4
}

kazi sogeza_kulia() -> Namba {
  rejesha 16 sogeza_kulia 2
}

kazi shift_negative_clamped() -> Namba {
  rejesha 8 sogeza_kushoto (0 - 1)
}

kazi shift_over_63_clamped() -> Namba {
  rejesha 1 sogeza_kushoto 64
}

kazi assign_plain() -> Namba {
  weka x = 42
  rejesha x
}

kazi assign_plus_equals_namba() -> Namba {
  weka n = 10
  n += 5
  rejesha n
}

kazi assign_plus_equals_neno() -> Neno {
  weka s = "a"
  s += "b"
  s += "c"
  rejesha s
}

kazi assign_minus_equals() -> Namba {
  weka n = 20
  n -= 7
  rejesha n
}

kazi assign_mul_equals() -> Namba {
  weka n = 6
  n *= 7
  rejesha n
}

kazi assign_div_equals() -> Namba {
  weka n = 100
  n /= 4
  rejesha n
}

kazi chain_comparison() -> Ukweli {
  rejesha 1 < 2 na 2 < 3
}

kazi eq_ukweli() -> Ukweli {
  rejesha kweli == kweli
}

kazi ne_ukweli() -> Ukweli {
  rejesha kweli != si_kweli
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  chapisha("--- Binary ops: arithmetic ---")
  chapisha("10+20: " + (add_namba() kama Neno))
  chapisha("hello+world: " + add_neno())
  chapisha("100-30: " + (sub_namba() kama Neno))
  chapisha("6*7: " + (mul_namba() kama Neno))
  chapisha("15/4: " + (div_namba() kama Neno))
  chapisha("17%5: " + (rem_namba() kama Neno))
  chapisha("2**10: " + (pow_namba() kama Neno))
  chapisha("--- Precedence ---")
  chapisha("1+2*3: " + (precedence_mul_before_add() kama Neno))
  chapisha("1+2<<2: " + (precedence_add_before_shift() kama Neno))
  chapisha("2*3**2: " + (precedence_pow_highest() kama Neno))
  chapisha("--- Equality ---")
  chapisha("5==5: " + (eq_namba() kama Neno))
  chapisha("a==a: " + (eq_neno() kama Neno))
  chapisha("3!=4: " + (ne_namba() kama Neno))
  chapisha("kweli==kweli: " + (eq_ukweli() kama Neno))
  chapisha("kweli!=si_kweli: " + (ne_ukweli() kama Neno))
  chapisha("--- Comparison Namba ---")
  chapisha("10>5: " + (gt_namba() kama Neno))
  chapisha("3<7: " + (lt_namba() kama Neno))
  chapisha("5>=5: " + (ge_namba() kama Neno))
  chapisha("4<=4: " + (le_namba() kama Neno))
  chapisha("--- Comparison Neno ---")
  chapisha("b>a: " + (gt_neno() kama Neno))
  chapisha("a<b: " + (lt_neno() kama Neno))
  chapisha("x>=x: " + (ge_neno() kama Neno))
  chapisha("x<=x: " + (le_neno() kama Neno))
  chapisha("Hamna==Hamna: " + (eq_hamna() kama Neno))
  chapisha("5**0: " + (pow_zero_exp() kama Neno))
  chapisha("--- Logical na/au ---")
  chapisha("kweli na kweli: " + (and_both_true() kama Neno))
  chapisha("kweli na si_kweli: " + (and_one_false() kama Neno))
  chapisha("si_kweli au si_kweli: " + (or_both_false() kama Neno))
  chapisha("kweli au si_kweli: " + (or_one_true() kama Neno))
  chapisha("1<2 na 2<3: " + (chain_comparison() kama Neno))
  chapisha("--- Bitwise ---")
  chapisha("3 na_biti 5: " + (na_biti() kama Neno))
  chapisha("3 au_biti 5: " + (au_biti() kama Neno))
  chapisha("3 xor_biti 5: " + (xor_biti() kama Neno))
  chapisha("1 sogeza_kushoto 4: " + (sogeza_kushoto() kama Neno))
  chapisha("16 sogeza_kulia 2: " + (sogeza_kulia() kama Neno))
  chapisha("8<<(-1) clamped: " + (shift_negative_clamped() kama Neno))
  chapisha("1<<64 clamped: " + (shift_over_63_clamped() kama Neno))
  chapisha("--- Assign ops ---")
  chapisha("weka x=42: " + (assign_plain() kama Neno))
  chapisha("n+=5: " + (assign_plus_equals_namba() kama Neno))
  chapisha("s+=b+=c: " + assign_plus_equals_neno())
  chapisha("n-=7: " + (assign_minus_equals() kama Neno))
  chapisha("n*=7: " + (assign_mul_equals() kama Neno))
  chapisha("n/=4: " + (assign_div_equals() kama Neno))
  chapisha("--- mwisho ---")
}
