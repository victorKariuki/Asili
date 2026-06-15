# Misingi ya Asili (Basics)

## Maoni (Comments)

```asili
# Hii ni maoni ya mstari mmoja
// Hii pia ni maoni
```

## Kutangaza Vigeuzi (Variable Declaration)

```asili
weka jina = "Amara"       # mutable variable
thabiti kasi = 3.0        # immutable constant
```

Variables must be declared with `weka` or `thabiti` before use. Reassignment works only on `weka` variables.

```asili
weka alama = 0
alama = alama + 10        # ok
thabiti PI = 3.14159
# PI = 3.0               # error: thabiti haiwezi kubadilishwa
```

## Aina za Msingi (Primitive Types)

| Aina      | Maelezo                          | Mfano                    |
|-----------|----------------------------------|--------------------------|
| `Namba`   | Floating-point number            | `3.14`, `100`, `-7.5`    |
| `Neno`    | UTF-8 string                     | `"Habari"`, `""`         |
| `Ukweli`  | Boolean                          | `kweli`, `si_kweli`      |
| `Herufi`  | Single Unicode character         | `'a'`, `'ñ'`             |
| `Tupu`    | No value (unit type)             | —                        |

### Aina za Namba Kamili (Integer Types)

| Aina     | Maelezo            |
|----------|--------------------|
| `Biti8`  | Signed 8-bit int   |
| `Biti16` | Signed 16-bit int  |
| `Biti32` | Signed 32-bit int  |
| `Biti64` | Signed 64-bit int  |
| `uBiti8` | Unsigned 8-bit int |
| `uBiti16`| Unsigned 16-bit    |
| `uBiti32`| Unsigned 32-bit    |
| `uBiti64`| Unsigned 64-bit    |

### Kutangaza Aina Wazi (Explicit Type Annotation)

```asili
weka umri: Namba = 25
weka nchi: Neno = "Kenya"
weka hai: Ukweli = kweli
```

## Kubadilisha Aina (Type Casting)

Use `kama` to cast between types:

```asili
weka n: Namba = 65
weka h = n kama Herufi      # 'A'
weka s = n kama Neno        # "65"
weka u = n kama Ukweli      # kweli (nonzero)

weka b: Biti8 = jaribu (200 kama Biti8)   # may fail: Chaguo<Biti8>
```

## Waendeshaji (Operators)

### Hesabu (Arithmetic)
```asili
weka a = 10 + 3   # 13
weka b = 10 - 3   # 7
weka c = 10 * 3   # 30
weka d = 10 / 3   # 3.333...
weka e = 10 % 3   # 1
weka f = 2 ** 8   # 256
```

### Linganisho (Comparison)
```asili
5 == 5      # kweli
5 != 3      # kweli
5 > 3       # kweli
5 < 3       # si_kweli
5 >= 5      # kweli
3 <= 5      # kweli
```

### Mantiki (Logical)
```asili
kweli na kweli      # kweli
kweli au si_kweli   # kweli
siyo kweli          # si_kweli
```

### Biti (Bitwise)
```asili
12 na_biti 10       # 8   (AND)
12 au_biti 3        # 15  (OR)
12 xor_biti 10      # 6   (XOR)
siyo_biti 0         # -1  (NOT)
1 sogeza_kushoto 3  # 8   (<<)
16 sogeza_kulia 2   # 4   (>>)
```

## Maoni ya Kujifunza

Utaratibu wa kuanza kufanya kazi na Asili:

1. Tumia `pata njozi mada` kuunda mradi mpya
2. Hariri `src/kuu.as`
3. Tumia `pata jenga --tenda` kuendesha
4. Tumia `pata repl` kujifunza mstari kwa mstari
