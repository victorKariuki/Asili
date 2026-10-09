# kidhibiti

A controller step written as strict code (`#[salama]`): `wastani` averages a fixed window of
samples, and `dhibiti` turns the error from a target into a correction clamped to ±10. Both keep
to the bounded subset (constant loop bounds, no recursion, no allocation, strict callees only),
so `pata jenga` checks them and prints their worst-case bounds:

```
$ pata jenga --tenda
salama: kazi 'dhibiti' — hatua ≤ 69, kumbukumbu ≤ baiti 200
salama: kazi 'wastani' — hatua ≤ 49, kumbukumbu ≤ baiti 104
msimbo asilia: ./kilele/kidhibiti.nguvu
imejengwa: ./kilele/kidhibiti.asb
1.5
```

See `docs/language/07-mfumo-wa-aina.md` ("Strict code") for the rules.
