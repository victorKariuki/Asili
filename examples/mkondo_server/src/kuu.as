leta mfumo
leta matumizi

# One request/response per connection: kazi_jina owns the whole connection lifecycle via the
# same .soma()/.andika()/.funga() methods mkondo_unganisha's client-side handle already exposes.
kazi mtumishi(m: Mkondo) -> Tupu {
    weka ombi = jaribu (m.soma())
    weka jibu = "umetuma bytes " + (ombi kama Neno)
    jaribu (m.andika(jibu))
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka sikilizaji = jaribu (mkondo_sikiliza("127.0.0.1:7878"))
    chapisha("inasikiliza kwenye 127.0.0.1:7878 (nyuzi 4)")
    # Blocks forever, joining the 4 worker threads it spawns — stop the process to stop serving.
    jaribu (mkondo_tumikia(sikilizaji, "mtumishi", 4.0))
}
