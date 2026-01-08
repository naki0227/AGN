use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = window)]
    fn updateScreen(s: &str);
}

#[wasm_bindgen(start)]
pub fn main() {
    // Generated from AGN source
    let ロゴ = format!("{\"type\":\"image\", \"src\":\"{}\"}", "logo.png");
    updateScreen(&format!("{}", ロゴ));
    let メインボタン = format!("{\"type\":\"component\", \"style\":\"Blue\", \"ty\":\"Button\"}");
    updateScreen(&format!("{}", メインボタン));
    let メッセージ = "AGN Phase 8 Demo";
    updateScreen(&format!("{}", メッセージ));
}
