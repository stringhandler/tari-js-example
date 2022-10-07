use std::collections::HashMap;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;
use web_sys::{Request, RequestInit, RequestMode, Response};
use wasm_bindgen::JsCast;
use serde::Serialize;
use serde::Deserialize;



// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
}

#[wasm_bindgen(js_name="sayHello")]
pub fn say_hello() -> Result<(), JsValue> {
    console::log_1(&JsValue::from_str("Hello 2"));
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct JsonRequest {
    jsonrpc: String,
    method: String,
    params: HashMap<String, String>,
    id: u32,
}

#[derive(Serialize, Deserialize)]
struct JsonRpcResponse<T> {
   id: u32,
    jsonrpc: String,
    result: T,
}

async fn make_json_request(url: String, method: String, params: HashMap<String, String>) -> Result<JsValue, JsValue> {

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);

    let s = JsonRequest{ jsonrpc: "2.0".to_string(), method: method, params: params, id: 1 };
    opts.body(Some(&JsValue::from_str(&serde_json::to_string(&s).unwrap())));

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request
        .headers()
        .set("Content-Type", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    // Send the JSON response back to JS.
    Ok(json)
}

#[wasm_bindgen]
pub struct TariConnection {
    url: String,
}

#[wasm_bindgen]
impl TariConnection {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String) -> TariConnection {
        TariConnection { url }
    }

    #[wasm_bindgen(js_name="getIdentity")]
    pub async fn get_identity(&self) -> Result<JsValue, JsValue> {
        let v = make_json_request(self.url.clone(), "get_identity".to_string(), HashMap::new()).await?;
        let res: JsonRpcResponse<GetIdentityResponse> = serde_wasm_bindgen::from_value(v)?;
        // console::log_1(&JsValue::from_str(res.result.public_address.as_str() ));
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }
}


#[derive(Serialize, Deserialize)]
struct GetIdentityResponse {
    node_id: String,
    public_key: String,
    public_address: String
}
