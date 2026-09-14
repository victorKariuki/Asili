leta mfumo
leta matumizi

umbo OmbiHttp { njia: Neno, anwani: Neno, vichwa: Kamusi<Neno, Neno>, mwili: Neno }
umbo JibuHttp { hali: Namba, vichwa: Kamusi<Neno, Neno>, mwili: Neno }

# kazi_jina for mkondo_tumikia_http: the framing layer owns parsing and response writing,
# this function only computes the response from the parsed request.
kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
    linganisha ombi.anwani {
        "/" => { rejesha JibuHttp { hali: 200, vichwa: kamusi(), mwili: "karibu kwenye seva ya Asili" } }
        "/echo" => { rejesha JibuHttp { hali: 200, vichwa: kamusi(), mwili: ombi.mwili } }
        _ => { rejesha JibuHttp { hali: 404, vichwa: kamusi(), mwili: "haipatikani: " + ombi.anwani } }
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka s = jaribu (mkondo_sikiliza("127.0.0.1:8080"))
    chapisha("HTTP inasikiliza kwenye 127.0.0.1:8080")
    jaribu (mkondo_tumikia_http(s, "mtumishi", 4.0))
}
