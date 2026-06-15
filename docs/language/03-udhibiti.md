# Udhibiti wa Mtiririko (Control Flow)

## Masharti (Conditionals)

### ikiwa / vinginevyo

```asili
weka alama = 75

ikiwa alama >= 90 {
  chapisha("Bora sana")
} au_ikiwa alama >= 60 {
  chapisha("Vizuri")
} vinginevyo {
  chapisha("Jaribu tena")
}
```

- `ikiwa` — if
- `au_ikiwa` — else if
- `vinginevyo` — else

### ikiwa kama Usemi (if as an Expression)

```asili
# (not supported as an expression — use au_ikiwa chains or linganisha instead)
```

## Linganisha (Pattern Matching)

`linganisha` matches a value against literal patterns. The `_` arm is the default/catch-all.

### Namba

```asili
weka n = 2
linganisha n {
  0 => { chapisha("sifuri") }
  1 => { chapisha("moja") }
  2 => { chapisha("mbili") }
  _ => { chapisha("nyingine") }
}
```

### Neno

```asili
weka rangi = "nyekundu"
linganisha rangi {
  "nyekundu" => { chapisha("Hatari") }
  "njano"    => { chapisha("Tahadhari") }
  "kijani"   => { chapisha("Salama") }
  _          => { chapisha("Rangi isiyojulikana") }
}
```

### Chaguo (Option Matching)

```asili
weka val = kamusi_tupu()
val.ingiza("x", 42)
weka pata = val.pata("z")

linganisha pata {
  Hamna => { chapisha("Hakuna thamani") }
  _     => { chapisha("Thamani inapatikana") }
}
```

## Mzunguko (Loops)

### wakati (while)

```asili
weka i = 0
wakati i < 5 {
  chapisha(i kama Neno)
  i = i + 1
}
```

### wakati milele (infinite loop)

```asili
wakati milele {
  weka ingizo = omba("Ingiza neno: ")
  ikiwa ingizo == "toka" { vunja }
  chapisha("Ulisema: " + ingizo)
}
```

### kwa katika (for-in over a collection)

```asili
weka namba = orodha(10, 20, 30, 40)
kwa x katika namba {
  chapisha(x kama Neno)
}
```

### kwa kutoka hadi (for range)

```asili
# 0 up to (not including) 5
kwa i kutoka 0 hadi 5 {
  chapisha(i kama Neno)
}
```

## Kudhibiti Mzunguko (Loop Control)

### vunja (break)

```asili
weka n = 0
wakati kweli {
  n = n + 1
  ikiwa n >= 3 { vunja }
}
# n == 3
```

### endelea (continue)

```asili
# Print only even numbers
kwa i kutoka 0 hadi 10 {
  ikiwa i % 2 != 0 { endelea }
  chapisha(i kama Neno)
}
```

### Lebo (Labels) for Nested Loops

```asili
lebo 'nje: wakati milele {
  kwa i kutoka 0 hadi 5 {
    ikiwa i == 3 { vunja 'nje }
  }
}
```

## Kutupa Thamani (Throwing Values)

`tupa` exits the current function by propagating a value as an error:

```asili
kazi gawio_salama(a: Namba, b: Namba) -> Namba {
  ikiwa b == 0 { tupa "Haiwezekani kugawanya na sifuri" }
  rejesha a / b
}
```
