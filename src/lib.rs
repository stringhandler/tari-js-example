use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;
use tari_crypto::keys::PublicKey;
use tari_crypto::ristretto::{RistrettoPublicKey, RistrettoSchnorr, RistrettoSecretKey};
use tari_crypto::tari_utilities::hex::Hex;
use tari_crypto::tari_utilities::ByteArray;
use tari_engine_types::hashing::hasher;
use tari_engine_types::instruction;
use tari_engine_types::signature::InstructionSignature;
use tari_template_lib::models::TemplateAddress;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;
use web_sys::{Request, RequestInit, RequestMode, Response};

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

#[wasm_bindgen(js_name = "sayHello")]
pub fn say_hello() -> Result<(), JsValue> {
    console::log_1(&JsValue::from_str("Hello 2"));
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct JsonRequest<T> {
    jsonrpc: String,
    method: String,
    params: T,
    id: u32,
}

#[derive(Serialize, Deserialize)]
struct JsonRpcResponse<T> {
    id: u32,
    jsonrpc: String,
    result: T,
}

async fn make_json_request<T: Serialize>(
    url: String,
    method: String,
    params: T,
) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);

    let s = JsonRequest {
        jsonrpc: "2.0".to_string(),
        method: method,
        params: params,
        id: 1,
    };
    let v = JsValue::from_str(&serde_json::to_string(&s).unwrap());
    console::log_1(&v);
    opts.body(Some(&v));

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request.headers().set("Content-Type", "application/json")?;

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

fn sign(
    secret_key: &RistrettoSecretKey,
    secret_nonce: RistrettoSecretKey,
    instructions: &[instruction::Instruction],
) -> InstructionSignature {
    InstructionSignature::sign(secret_key, secret_nonce, instructions)
    // let public_key = RistrettoPublicKey::from_secret_key(secret_key);
    //
    // let (nonce, nonce_pk) = RistrettoPublicKey::random_keypair(&mut OsRng) ;
    // // TODO: implement dan encoding for (a wrapper of) PublicKey
    // let challenge = hasher("instruction-signature")
    //     .chain(nonce_pk.as_bytes())
    //     .chain(public_key.as_bytes())
    //     .chain(instructions)
    //     .result();
    // RistrettoSchnorr::sign(secret_key.clone(), nonce, &challenge).unwrap()
}

#[wasm_bindgen]
pub struct TariConnection {
    url: String,
    // Lol, best remove this at some point
    secret_key: RistrettoSecretKey,
}

#[wasm_bindgen]
impl TariConnection {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, secret_key_hex: String) -> TariConnection {
        TariConnection {
            url,
            secret_key: RistrettoSecretKey::from_hex(&secret_key_hex).unwrap(),
        }
    }

    #[wasm_bindgen(js_name = "getIdentity")]
    pub async fn get_identity(&self) -> Result<JsValue, JsValue> {
        let v = make_json_request(
            self.url.clone(),
            "get_identity".to_string(),
            HashMap::<String, String>::new(),
        )
        .await?;
        let res: JsonRpcResponse<GetIdentityResponse> = serde_wasm_bindgen::from_value(v)?;
        // console::log_1(&JsValue::from_str(res.result.public_address.as_str() ));
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }

    #[wasm_bindgen(js_name = "getTemplates")]
    pub async fn get_templates(&self, limit: Option<u32>) -> Result<JsValue, JsValue> {
        let v = make_json_request(
            self.url.clone(),
            "get_templates".to_string(),
            GetTemplatesRequest { limit: limit.unwrap_or(20) },
        )
            .await?;
        console::log_1(&v);
        let res: JsonRpcResponse<GetTemplatesResponse> = serde_wasm_bindgen::from_value(v)?;
        // console::log_1(&JsValue::from_str(res.result.public_address.as_str() ));
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }

    #[wasm_bindgen(js_name = "getTemplate")]
    pub async fn get_template(&self, address: String) -> Result<JsValue, JsValue> {
        let v = make_json_request(
            self.url.clone(),
            "get_template".to_string(),
            GetTemplateRequest {template_address: address },
        )
            .await?;
        console::log_1(&v);
        let res: JsonRpcResponse<GetTemplateResponse> = serde_wasm_bindgen::from_value(v)?;
        // console::log_1(&JsValue::from_str(res.result.public_address.as_str() ));
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }

    #[wasm_bindgen(js_name = "submitFunctionCall")]
    pub async fn submit_function_call(
        &self,
        template_address: String,
        method: String,
    ) -> Result<JsValue, JsValue> {
        // let (secret_key, public_key) = RistrettoPublicKey::random_keypair(&mut OsRng) ;
        // let instructions = vec![Instruction{
        //     template_address,
        //     method,
        //     args: vec![]
        // }];
        let instruction = instruction::Instruction::CallFunction {
            template_address: TemplateAddress::from_hex(&template_address).unwrap(),
            function: method.clone(),
            args: vec![],
        };
        let instructions = vec![instruction];
        // TODO: lol better pls
        let sec_nonce = RistrettoSecretKey::from_bytes(&[1u8; 32]).unwrap();
        let pub_nonce = RistrettoPublicKey::from_secret_key(&sec_nonce);
        let signature = sign(&self.secret_key, sec_nonce, &instructions);

        // let challenge = sign(secret_key, public_key, instructions);
        let req = SubmitTransactionRequest {
            instructions: vec![Instruction{
                type_: "CallFunction".to_string(),
                template_address,
                function: method,
                args: vec![]
            }],
            signature: Signature {
                signature: signature.signature().get_signature().to_hex(),
                public_nonce: pub_nonce.to_hex(),
            },
            fee: 0,
            sender_public_key: RistrettoPublicKey::from_secret_key(&self.secret_key).to_hex(),
            num_new_components: 2,
        };
        let v = make_json_request(self.url.clone(), "submit_transaction".to_string(), req).await?;

        console::log_1(&v);
        let res: JsonRpcResponse<SubmitTransactionResponse> = serde_wasm_bindgen::from_value(v)?;
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTemplateRequest {
    template_address: String
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTemplateResponse {
    registration_metadata: TemplateMetadata,
    abi: TemplateAbi
}

#[derive(Serialize, Deserialize, Debug)]
struct TemplateAbi {
    template_name: String,
    functions: Vec<FunctionDef>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub arguments: Vec<String>,
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTemplatesRequest {
    limit: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTemplatesResponse {
    templates: Vec<TemplateMetadata>,

}
#[derive(Serialize, Deserialize, Debug)]
struct TemplateMetadata {
    address: String,
    url: String,
    binary_sha: Vec<u8>,
    height: u32
}


#[derive(Serialize, Deserialize)]
struct GetIdentityResponse {
    node_id: String,
    public_key: String,
    public_address: String,
}

#[derive(Serialize, Deserialize)]
struct SubmitTransactionRequest {
    instructions: Vec<Instruction>,
    signature: Signature,
    fee: u64,
    sender_public_key: String,
    num_new_components: u64,
}

#[derive(Serialize, Deserialize)]
struct Signature {
    signature: String,
    public_nonce: String,
}

#[derive(Serialize, Deserialize)]
struct Instruction {
    #[serde(rename = "type")]
    type_: String,
    template_address: String,
    function: String,
    args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct SubmitTransactionResponse {
    hash: String,
}
