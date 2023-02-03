use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct TransactionBuilder {

}

#[wasm_bindgen]
impl TransactionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }
}
