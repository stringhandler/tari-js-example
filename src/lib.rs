use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;
use js_sys::Array;
use tari_crypto::keys::PublicKey;
use tari_crypto::ristretto::{RistrettoPublicKey, RistrettoSchnorr, RistrettoSecretKey};
use tari_crypto::tari_utilities::ByteArray;
use tari_crypto::tari_utilities::hex::Hex;
use tari_engine_types::commit_result::TransactionResult;
use tari_engine_types::hashing::hasher;
use tari_engine_types::instruction;
use tari_engine_types::instruction::Instruction;
use tari_engine_types::substate::SubstateAddress;
use tari_template_lib::args;
use tari_transaction::InstructionSignature;
use tari_template_lib::models::TemplateAddress;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, Window};
use web_sys::{Request, RequestInit, RequestMode, Response};
use tari_template_lib::models::ComponentAddress;
use tari_template_lib::args::Arg;
use tari_template_lib::constants::PUBLIC_IDENTITY_RESOURCE;
use web_sys::WorkerGlobalScope;
use tari_validator_node_client::types::SubmitTransactionRequest;
use crate::transaction_builder::TransactionBuilder;
use tari_validator_node_client::types::SubmitTransactionResponse;
use tari_transaction::Transaction;
use tari_template_lib::prelude::NonFungibleAddress;
use tari_template_lib::crypto::RistrettoPublicKeyBytes;
use std::fmt::Write;

mod transaction_builder;


// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// #[wasm_bindgen(module="/wasm-imports.js")]
// extern "C" {
//    fn fetch()
// }

// thread_local! {
//     static GLOBAL: WindowOrWorker = WindowOrWorker::new();
// }

enum WindowOrWorker {
    Window(Window),
    Worker(WorkerGlobalScope),
}

impl WindowOrWorker {
    fn new() -> Self {
        #[wasm_bindgen]
        extern "C" {
            type Global;

            #[wasm_bindgen(method, getter, js_name = Window)]
            fn window(this: &Global) -> JsValue;

            #[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
            fn worker(this: &Global) -> JsValue;
        }

        let global: Global = js_sys::global().unchecked_into();

        if !global.window().is_undefined() {
            Self::Window(global.unchecked_into())
        } else if !global.worker().is_undefined() {
            Self::Worker(global.unchecked_into())
        } else {
            panic!("Only supported in a browser or web worker");
        }
    }
}

impl WindowOrWorker {
  fn as_window(&self) -> Option<&Window> {
    match self {
      Self::Window(window) => Some(window),
      _ => None,
    }
  }

    fn as_worker(&self) -> Option<&WorkerGlobalScope> {
        match self {
            Self::Worker(worker) => Some(worker),
            _ => None,
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = crypto, js_name = "getRandomValues")]
    fn get_random_values(buf: &mut [u8]);
}


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
    error: Option<JsonRpcError>
}

#[derive(Serialize, Deserialize)]
struct JsonRpcError {
    code: u32,
    message: String,
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
    let resp_value;
    let window_or_worker = WindowOrWorker::new();
    if let Some(window) = window_or_worker.as_window() {
        resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    } else {
        if let Some(worker) = window_or_worker.as_worker() {
            resp_value = JsFuture::from(worker.fetch_with_request(&request)).await?;
        } else {
            panic!("Only supported in a browser or web worker");
        }
    }


    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    // Send the JSON response back to JS.
    Ok(json)
}

fn to_hex(slice: &[u8]) -> String {
    let mut s = String::with_capacity(2 * slice.len());
    for byte in slice {
        write!(s, "{:02X}", byte).expect("could not write hex");
    }
    s
}


#[wasm_bindgen]
pub struct TariConnection {
    url: String,
    // Lol, best remove this at some point
    secret_key: RistrettoSecretKey,
    public_key: RistrettoPublicKey,
}

#[wasm_bindgen]
impl TariConnection {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, secret_key_hex: String) -> TariConnection {
        let secret_key = RistrettoSecretKey::from_hex(&secret_key_hex).unwrap();
        TariConnection {
            url,
            public_key: RistrettoPublicKey::from_secret_key(&secret_key),
            secret_key,
        }
    }

    #[wasm_bindgen(js_name = "getPublicKey")]
    pub fn get_public_key(&self) -> JsValue {
        JsValue::from_str(&self.public_key.to_hex())
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

    #[wasm_bindgen(js_name = "createAccount")]
    pub async fn create_account(
        &self
    ) -> Result<JsValue, JsValue> {
        let resource_address = PUBLIC_IDENTITY_RESOURCE;
        // TODO: I manually create the address, but you can use a NonFungibleAddress
        // instead
        let mut vec = resource_address.hash().to_vec();
        // enum type
        vec.extend_from_slice(&[0]);
        vec.extend_from_slice(&self.public_key.as_bytes());

        let instruction = Instruction::CallFunction {
            template_address: TemplateAddress::from([0u8;32]),
            function: "create".to_string(),
            // args: args.iter().map(|js| Arg::Literal(Vec::<u8>::from_hex(&js.as_string().unwrap()).unwrap())).collect(),
            // args: args![NonFungibleAddress::from_bytes(RistrettoPublicKeyBytes::from(self.public_key.as_bytes())).to_hex()],
            args: vec![Arg::Literal(vec)]
        };

        let mut bytes= vec![0u8;32];
        get_random_values(&mut bytes);
        let sec_nonce = RistrettoSecretKey::from_bytes(&bytes).unwrap();
        // let pub_nonce = RistrettoPublicKey::from_secret_key(&sec_nonce);
        // let signature = sign(, sec_nonce, &instructions);

        console::log_1(&JsValue::from_str("instruction created"));
        let mut builder = Transaction::builder();
        console::log_1(&JsValue::from_str("instruction created 1"));
        builder.add_instruction(instruction).with_new_outputs(5);
        console::log_1(&JsValue::from_str("instruction created 2"));
        builder.sign_with_nonce(&self.secret_key, sec_nonce);
        console::log_1(&JsValue::from_str("instruction signed"));
        let transaction = builder.build();
        let expected_component = transaction.meta().involved_shards().first().unwrap().clone();
        // let challenge = sign(secret_key, public_key, instructions);
        let req = SubmitTransactionRequest {
            transaction,
            is_dry_run: false,
            wait_for_result: true,
            wait_for_result_timeout: Some(60)

        };
        let v = make_json_request(self.url.clone(), "submit_transaction".to_string(), req).await?;

        console::log_1(&v);
        let res: JsonRpcResponse<SubmitTransactionResponse> = serde_wasm_bindgen::from_value(v)?;
        if res.error.is_some() {
            return Err(JsValue::from_str(&res.error.unwrap().message));
        }
        let component_address;
        match res.result.result.unwrap().finalize.result {
            TransactionResult::Accept(substate_diff) => {
                for (address, diff) in substate_diff.up_iter() {
                   match address {
                    SubstateAddress::Component(addr) =>  {
                        component_address = addr.clone();
                        console::log_1(&JsValue::from_str(&format!("component address: {}", address)));
                        return Ok(serde_wasm_bindgen::to_value(&to_hex(component_address.hash()))?);
                    } ,
                    _ => {}
                   }
                }
            }
            TransactionResult::Reject(reason) => {
                return Err(JsValue::from_str(&format!("Transaction rejected:{}", reason)));
            }
        }

        Err(JsValue::from_str("No component address found"))

    }


    #[wasm_bindgen(js_name = "submitFunctionCall")]
    pub async fn submit_function_call(
        &self,
        template_address: String,
        method: String,
        args: Array,
        wait_for_result: bool
    ) -> Result<JsValue, JsValue> {
        console::log_1(&JsValue::from_str("Entered submit function call"));
        let instruction = Instruction::CallFunction {
            template_address: TemplateAddress::from_hex(&template_address).unwrap(),
            function: method.clone(),
            // args: args.iter().map(|js| Arg::Literal(Vec::<u8>::from_hex(&js.as_string().unwrap()).unwrap())).collect(),
            args: args.iter().map(|js| Arg::Literal(js.as_string().unwrap().as_bytes().to_vec())).collect(),
        };
        // TODO: lol better pls

        let mut bytes= vec![0u8;32];
        get_random_values(&mut bytes);
        let sec_nonce = RistrettoSecretKey::from_bytes(&bytes).unwrap();
        // let pub_nonce = RistrettoPublicKey::from_secret_key(&sec_nonce);
        // let signature = sign(, sec_nonce, &instructions);

        console::log_1(&JsValue::from_str("instruction created"));
        let mut builder = Transaction::builder();
        console::log_1(&JsValue::from_str("instruction created 1"));
        builder.add_instruction(instruction).with_new_outputs(5);
        console::log_1(&JsValue::from_str("instruction created 2"));
        builder.sign_with_nonce(&self.secret_key, sec_nonce);
        console::log_1(&JsValue::from_str("instruction signed"));
        let transaction = builder.build();
        // let challenge = sign(secret_key, public_key, instructions);
        let req = SubmitTransactionRequest {
            transaction,
            is_dry_run: false,
            wait_for_result,
            wait_for_result_timeout: Some(60)

        };
        let v = make_json_request(self.url.clone(), "submit_transaction".to_string(), req).await?;

        console::log_1(&v);
        let res: JsonRpcResponse<SubmitTransactionResponse> = serde_wasm_bindgen::from_value(v)?;
        if res.error.is_some() {
            return Err(JsValue::from_str(&res.error.unwrap().message));
        }
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }

    #[wasm_bindgen(js_name = "submitMethodCall")]
    pub async fn submit_method_call(
        &self,
        template_address: String,
        component_address: String,
        method: String,
        args: Array,
        wait_for_result: bool
    ) -> Result<JsValue, JsValue> {
        let instruction = Instruction::CallMethod {
            // template_address: TemplateAddress::from_hex(&template_address).unwrap(),
            component_address: ComponentAddress::from_hex(&component_address).unwrap(),
            method: method.clone(),
            args: args.iter().map(|a| Arg::Literal(Vec::from_hex(&a.as_string().unwrap()).unwrap())).collect(),
            // args: vec![],
        };

        let mut builder = Transaction::builder();
        builder.add_instruction(instruction).with_new_outputs(5);
        builder.sign(&self.secret_key);
        let transaction = builder.build();
        // let challenge = sign(secret_key, public_key, instructions);
        let req = SubmitTransactionRequest {
         transaction,
            is_dry_run: false,
            wait_for_result,
            wait_for_result_timeout: Some(60)

        };
        let v = make_json_request(self.url.clone(), "submit_transaction".to_string(), req).await?;

        console::log_1(&v);
        let res: JsonRpcResponse<SubmitTransactionResponse> = serde_wasm_bindgen::from_value(v)?;
        if res.error.is_some() {
            return Err(JsValue::from_str(&res.error.unwrap().message));
        }
        Ok(serde_wasm_bindgen::to_value(&res.result)?)
    }

    pub async fn call_method_and_deposit_buckets(   &self,
                                              component_address: String,
                                                    account_address: String,
                                              method: String,
                                              args: Array,
    ) -> Result<JsValue, JsValue> {
        let instruction = Instruction::CallMethod {
            // template_address: TemplateAddress::from_hex(&template_address).unwrap(),
            // template_address: Default::default(),
            component_address: ComponentAddress::from_hex(&component_address).unwrap(),
            method: method.clone(),
            args: args.iter().map(|a| Arg::Literal(Vec::from_hex(&a.as_string().unwrap()).unwrap())).collect(),
        };


        let mut builder = Transaction::builder();
        builder.add_instruction(instruction)
            .add_instruction(Instruction::PutLastInstructionOutputOnWorkspace { key: b"bucket".to_vec() })
            .add_instruction(Instruction::CallMethod {
                // template_address: TemplateAddress::from_hex([0u8; 32].to_hex().as_str()).unwrap(),
                component_address: ComponentAddress::from_hex(&account_address).unwrap(),
                method: "deposit".to_string(),
                args: vec![Arg::Variable(b"bucket".to_vec())],
            }).with_new_outputs(5);
        builder.sign(&self.secret_key);
        let transaction = builder.build();

        // let challenge = sign(secret_key, public_key, instructions);
        let req = SubmitTransactionRequest {
            transaction,
            wait_for_result: true,
            is_dry_run: false,
            wait_for_result_timeout: Some(60)
        };
        let v = make_json_request(self.url.clone(), "submit_transaction".to_string(), req).await?;

        console::log_1(&v);
        let res: JsonRpcResponse<SubmitTransactionResponse> = serde_wasm_bindgen::from_value(v)?;
        if res.error.is_some() {
            return Err(JsValue::from_str(&res.error.unwrap().message));
        }
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

// #[derive(Serialize, Deserialize)]
// struct SubmitTransactionRequest {
//     instructions: Vec<Instruction>,
//     signature: Signature,
//     fee: u64,
//     sender_public_key: String,
//     num_new_components: u64,
//     wait_for_result: bool
// }

// #[derive(Serialize, Deserialize)]
// struct Signature {
//     signature: String,
//     public_nonce: String,
// }

// #[derive(Serialize, Deserialize)]
// struct Instruction {
//     #[serde(rename = "type")]
//     type_: String,
//     template_address: String,
//     component_address: Option<String>,
//     function: Option<String>,
//     method: Option<String>,
//     args: Vec<String>,
// }

// #[derive(Serialize, Deserialize)]
// struct SubmitTransactionResponse {
//     hash: String,
//     // changes: serde_json::Value,
// }
