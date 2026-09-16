// Static file server for the Asili Playground, written in Asili itself — serves the same
// index.html/style.css/main.js/pkg/samples that examples/playground's README also shows how to
// serve with a plain `python3 -m http.server`, using this repo's own HTTP framing layer
// (mkondo_tumikia_http, see examples/http_server) instead. Run with `pata jenga --tenda` from
// this directory (examples/playground), then open http://127.0.0.1:8080/.
//
// Asili's soma_faili reads text (UTF-8) files only — no raw-bytes file API yet — so the wasm
// binary is served pre-encoded as base64 text (see build.sh's `xxd`/`base64` step, producing
// pkg/asili_wasm_bg.wasm.b64) and decoded back to bytes by main.js before instantiating it.
leta mfumo
leta matumizi
leta faili

umbo OmbiHttp { njia: Neno, anwani: Neno, vichwa: Kamusi<Neno, Neno>, mwili: Neno }
umbo JibuHttp { hali: Namba, vichwa: Kamusi<Neno, Neno>, mwili: Neno }

kazi jibu_faili(njia: Neno, aina: Neno) -> JibuHttp {
    linganisha soma_faili(njia.clona()) {
        Tokeo::Sawa(maudhui) => {
            weka vichwa = kamusi_tupu()
            vichwa.ingiza("Content-Type", aina)
            rejesha JibuHttp { hali: 200, vichwa: vichwa, mwili: maudhui }
        }
        Tokeo::Kosa(_) => {
            rejesha JibuHttp { hali: 404, vichwa: kamusi_tupu(), mwili: "haipatikani: " + njia }
        }
    }
}

// kazi_jina for mkondo_tumikia_http: routes each request path to the matching static asset.
kazi mtumishi(ombi: OmbiHttp) -> JibuHttp {
    linganisha ombi.anwani {
        "/" => { rejesha jibu_faili("index.html", "text/html; charset=utf-8") }
        "/index.html" => { rejesha jibu_faili("index.html", "text/html; charset=utf-8") }
        "/style.css" => { rejesha jibu_faili("style.css", "text/css; charset=utf-8") }
        "/main.js" => { rejesha jibu_faili("main.js", "text/javascript; charset=utf-8") }
        "/pkg/asili_wasm.js" => { rejesha jibu_faili("pkg/asili_wasm.js", "text/javascript; charset=utf-8") }
        "/pkg/asili_wasm_bg.wasm.b64" => { rejesha jibu_faili("pkg/asili_wasm_bg.wasm.b64", "text/plain; charset=utf-8") }
        "/samples/manifest.json" => { rejesha jibu_faili("samples/manifest.json", "application/json; charset=utf-8") }
        "/samples/karibu.as" => { rejesha jibu_faili("samples/karibu.as", "text/plain; charset=utf-8") }
        "/samples/sifa.as" => { rejesha jibu_faili("samples/sifa.as", "text/plain; charset=utf-8") }
        "/samples/makosa.as" => { rejesha jibu_faili("samples/makosa.as", "text/plain; charset=utf-8") }
        _ => { rejesha JibuHttp { hali: 404, vichwa: kamusi_tupu(), mwili: "haipatikani: " + ombi.anwani } }
    }
}

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
    weka s = jaribu (mkondo_sikiliza("127.0.0.1:8080"))
    chapisha("Asili Playground inasikiliza kwenye http://127.0.0.1:8080")
    jaribu (mkondo_tumikia_http(s, "mtumishi", 4.0))
}
