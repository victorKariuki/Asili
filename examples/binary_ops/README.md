# Binary and assign operators — pushed to limits

Covers every binary operator and assign operator in Asili so none are overlooked.

## Binary operators (expression)

| Operator | Keyword / symbol | Example function | Notes |
|----------|------------------|------------------|--------|
| Add | `+` | add_namba, add_neno | Namba or Neno (concat) |
| Sub | `-` | sub_namba | Namba only |
| Mul | `*` | mul_namba | Namba only |
| Div | `/` | div_namba | Namba only; div by zero → abort |
| Rem | `%` | rem_namba | Namba only |
| Pow | `**` | pow_namba, pow_zero_exp | Namba only |
| Eq | `==` | eq_namba, eq_neno, eq_ukweli, eq_hamna | Any value |
| Ne | `!=` | ne_namba, ne_neno, ne_ukweli | Any value |
| Gt | `>` | gt_namba, gt_neno | Namba or Neno |
| Lt | `<` | lt_namba, lt_neno | Namba or Neno |
| Ge | `>=` | ge_namba, ge_neno | Namba or Neno |
| Le | `<=` | le_namba, le_neno | Namba or Neno |
| And | `na` | and_both_true, and_one_false, chain_comparison | Ukweli; short-circuit |
| Or | `au` | or_both_false, or_one_true | Ukweli; short-circuit |
| BitAnd | `na_biti` | na_biti | Namba (i64) |
| BitOr | `au_biti` | au_biti | Namba (i64) |
| BitXor | `xor_biti` | xor_biti | Namba (i64) |
| Shl | `sogeza_kushoto` | sogeza_kushoto, shift_negative_clamped, shift_over_63_clamped | Shift 0..63 |
| Shr | `sogeza_kulia` | sogeza_kulia | Shift 0..63 |

## Assign operators (statement)

| Operator | Symbol | Example function |
|----------|--------|------------------|
| Assign | `=` | assign_plain |
| AddAssign | `+=` | assign_plus_equals_namba, assign_plus_equals_neno |
| SubAssign | `-=` | assign_minus_equals |
| MulAssign | `*=` | assign_mul_equals |
| DivAssign | `/=` | assign_div_equals |

## Precedence (demonstrated)

- `1+2*3` → 7 (mul before add)
- `1+2 sogeza_kushoto 2` → 12 (term before shift: (1+2)<<2)
- `2*3**2` → 18 (pow before mul)

## Edge cases

- Neno `+` concatenation (add_neno, assign_plus_equals_neno)
- Shift amount negative → clamped to 0 (shift_negative_clamped)
- Shift amount > 63 → clamped to 0 (shift_over_63_clamped)
- Eq on Hamna (eq_hamna)
- 5**0 (pow_zero_exp)
- Ge/Le on Neno (ge_neno, le_neno)

## Not run in kuu (would abort or require special handling)

- Division by zero (`/` or `/=` with 0)
- Short-circuit: `kweli au paparika("")` and `si_kweli na paparika("")` (tested in integration tests)

Run: `cd examples/binary_ops && cargo run -p pata-cli -- jenga --tenda`
