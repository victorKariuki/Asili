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

## On a device

`pata jenga --lengo cortex-m` also builds both functions for Cortex-M (Thumb-2, `fpv5-d16`):
`kilele/kidhibiti-cortex-m.o` and `kilele/kidhibiti.h`, for a firmware to link with the device
runtime (`driver/kifaa`). From C:

```c
asili_dhibiti_hoja hoja = { { sampuli, 8 }, 5.0 };
double marekebisho;
if (asili_dhibiti(&hoja, &marekebisho) != 0) { /* safe state */ }
```

See `docs/howto/07-build-for-cortex-m.md`.
