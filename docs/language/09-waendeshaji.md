# Waendeshaji (Operators) — Mpangilio na Maelezo Kamili

## Utaratibu wa Kipaumbele (Precedence — high to low)

| Kiwango | Waendeshaji                                          | Ushirikiano |
|---------|------------------------------------------------------|-------------|
| 1 (juu) | `-x`, `siyo x`, `siyo_biti x`, `jaribu x`, `azima x`, `azima_tenda x` | Kulia→Kushoto |
| 2       | `**`                                                 | Kulia→Kushoto |
| 3       | `*`, `/`, `%`                                        | Kushoto→Kulia |
| 4       | `+`, `-`                                             | Kushoto→Kulia |
| 5       | `sogeza_kushoto`, `sogeza_kulia`                     | Kushoto→Kulia |
| 6       | `<`, `>`, `<=`, `>=`                                 | Kushoto→Kulia |
| 7       | `==`, `!=`                                           | Kushoto→Kulia |
| 8       | `na_biti`                                            | Kushoto→Kulia |
| 9       | `xor_biti`                                           | Kushoto→Kulia |
| 10      | `au_biti`                                            | Kushoto→Kulia |
| 11      | `na`                                                 | Kushoto→Kulia |
| 12      | `au`                                                 | Kushoto→Kulia |
| 13      | `kama` (cast)                                        | Kushoto→Kulia |
| 14 (chini) | `=`, `+=`, `-=`, `*=`, `/=`                       | Kulia→Kushoto |

### Mifano ya Kipaumbele

```asili
1 + 2 * 3         # 7  — * kabla ya +
2 * 3 ** 2        # 18 — ** kabla ya *
-2 * 3            # -6 — unary - kabla ya *
1 + 2 sogeza_kushoto 2   # 12 — + kabla ya <<
10 > 5 na 3 < 7   # kweli — > na < kabla ya na
```

---

## Waendeshaji wa Upande Mmoja (Unary Operators)

| Waendeshaji   | Maelezo                                    | Mfano                       |
|---------------|--------------------------------------------|-----------------------------|
| `-x`          | Negation (negate)                          | `-5` → `-5`                |
| `siyo x`      | Logical NOT                                | `siyo kweli` → `si_kweli`  |
| `siyo_biti x` | Bitwise NOT (two's complement)             | `siyo_biti 0` → `-1`       |
| `jaribu x`    | Unwrap `Tokeo` / `Chaguo`, propagate error | `jaribu gawio(10, 2)` → `5`|
| `azima x`     | Immutable borrow (reference)               | `azima x`                   |
| `azima_tenda x` | Mutable borrow                           | `azima_tenda x`             |

### Waendeshaji wa Upande Mmoja waweza kushirikiana

```asili
- - 1             # 1   (double negation)
- - - 1           # -1  (triple negation)
siyo siyo kweli   # kweli
siyo siyo siyo kweli  # si_kweli
siyo_biti (1 + 2) # -4  (bitwise NOT of 3)
- siyo_biti 0     # 1   (negate of bitwise NOT of 0)
10 - - 1          # 11  (binary minus then unary minus)
```

### `jaribu` kama Waendeshaji wa Upande Mmoja

`jaribu` mbele ya usemi unaorudisha `Tokeo` au `Chaguo` — hutoa thamani au kueneza kosa:

```asili
weka matokeo = jaribu gawio(10, 2)      # 5.0
weka b8 = jaribu (200 kama Biti8)       # Chaguo unwrap
```

---

## Maadili Maalum ya Namba (Special Numeric Values)

| Jina        | Maelezo                          | Mfano                          |
|-------------|----------------------------------|--------------------------------|
| `Ukomo`     | Positive infinity (`+∞`)        | `1 / 0` → `Ukomo`            |
| `-Ukomo`    | Negative infinity (`-∞`)        | `-1 / 0` → `-Ukomo`          |
| `Siyo_Namba`| NaN (Not a Number)              | `0 / 0` → `Siyo_Namba`       |

```asili
leta hisabati

ni_ukomo(Ukomo)         # kweli
ni_ukomo(-Ukomo)        # kweli
si_namba(Siyo_Namba)    # kweli
ni_namba(Siyo_Namba)    # si_kweli
ni_namba(42)            # kweli

# NaN is not equal to itself
Siyo_Namba == Siyo_Namba   # si_kweli  (IEEE 754 behaviour)
```

---

## Waendeshaji wa Neno (String Operators)

| Waendeshaji | Maelezo                              | Mfano                          |
|-------------|--------------------------------------|--------------------------------|
| `+`         | Concatenation                        | `"a" + "b"` → `"ab"`         |
| `+=`        | Append in place                      | `s += "!"` — appends to `s`  |
| `==`, `!=`  | Lexicographic equality               | `"abc" == "abc"` → `kweli`   |
| `<`, `>` etc| Lexicographic comparison             | `"b" > "a"` → `kweli`        |

```asili
weka s = "Habari"
s += ", "
s += "Dunia!"
# s == "Habari, Dunia!"

"café" > "cafe"   # kweli (é > e lexicographically)
```

---

## Waendeshaji wa Biti (Bitwise Operators)

All bitwise operators work on the integer representation of `Namba`:

| Waendeshaji      | Maelezo              | Mfano                        |
|------------------|----------------------|------------------------------|
| `na_biti`        | AND                  | `3 na_biti 5` → `1`         |
| `au_biti`        | OR                   | `3 au_biti 5` → `7`         |
| `xor_biti`       | XOR                  | `3 xor_biti 5` → `6`        |
| `siyo_biti`      | NOT (bitwise)        | `siyo_biti 0` → `-1`        |
| `sogeza_kushoto` | Left shift (`<<`)    | `1 sogeza_kushoto 4` → `16` |
| `sogeza_kulia`   | Right shift (`>>`)   | `16 sogeza_kulia 2` → `4`   |

### Mipaka ya Mabadiliko (Shift Clamping)

Shifts outside the range `[0, 63]` are clamped silently:

```asili
1 sogeza_kushoto 64   # 0  (clamped to 0 shift)
8 sogeza_kushoto -1   # 8  (negative shift clamped to 0)
```

### Utekelezaji wa Biti kwa Vitendo

```asili
# Check if a bit is set
weka flags = 0b1010
weka bit_1_set = (flags na_biti 0b0010) != 0   # kweli

# Set a bit
flags = flags au_biti 0b0001   # 0b1011

# Toggle a bit
flags = flags xor_biti 0b1000  # 0b0011

# Clear a bit
flags = flags na_biti (siyo_biti 0b0010)  # 0b0001
```

---

## Waendeshaji wa Muundo wa Kuchanganya (Compound Assignment)

```asili
weka n = 10
n += 5    # 15
n -= 3    # 12
n *= 2    # 24
n /= 4    # 6

weka s = "Hello"
s += " World"   # "Hello World"  (works on Neno too)
```

---

## Waendeshaji wa Ufikiaji (Access Operators)

| Waendeshaji | Maelezo                                  | Mfano           |
|-------------|------------------------------------------|-----------------|
| `.`         | Field or method access                   | `mtu.jina`      |
| `[]`        | Index access                             | `a[0]`          |
| `[]?`       | Index with error propagation             | `a[0]?`         |
| `::`        | Enum variant or namespace path           | `Rangi::Nyekundu` |
| `?`         | Error propagation (postfix `?`)          | `a[i]?`         |
| `->`        | Return type annotation (not an operator) | `kazi f() -> Namba` |
