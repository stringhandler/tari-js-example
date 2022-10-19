import axios from "axios";



function toJsonRpc(method, params){
    return {
        jsonrpc: '2.0',
        method,
        params,
        id: 1,
    }
}

//
// class TariConnectionJs {
//
//     // private kv: KeyRing;
//     constructor(url) {
//         this.url = url
//         // init
//
//         // this.kv = KeyRing.new();
//     }
//
//     async getIdentity() {
//         return (await axios({
//             method: 'post',
//             url: this.url,
//             data: toJsonRpc("get_identity", []),
//
//         })).data.result
//     }
// }


// use tari_template_macros::template;
//
// #[template]
// mod counter {
//     pub struct Counter {
//         value: u32,
//     }
//
//     impl Counter {
//         pub fn new() -> Self {
//             Self { value: 0 }
//         }
//
//         pub fn value(&self) -> u32 {
//             self.value
//         }
//
//         pub fn increase(&mut self) {
//             self.value += 1;
//         }
//     }
// }

function setError(e) {
    document.getElementById("alert").innerText = e;
}

function clearError() {
    document.getElementById("alert").innerText = "";
    document.getElementById("result").innerText = "";
}

async function main() {
    try {
        const tari = await import("../pkg/index.js").catch(console.error);
        window.tariLib = tari;
        let conn = new tari.TariConnection("http://localhost:18200/json_rpc", "222f59f59229dda1f3453a143b12eb2947e2217664be1373a85b25d9cb3ab642");
        let id = await conn.getIdentity();
        console.log(id);
        let templates = await conn.getTemplates(10);
        console.log(templates);
        let x = document.getElementById("templates");
        for (let template of templates.templates){
            let x = document.getElementById("templates");
            var option = document.createElement("option");
            option.text = template.address;
            x.add(option);
        }
        let hTemplateName =document.getElementById("templateName");
        let ddFunc = document.getElementById("func");
        x.onchange = async function() {
            try {
                clearError();
                window.templateAddress = x.value;
                window.template = await conn.getTemplate(x.value);
                console.log(template);
                hTemplateName.innerText = template.abi.template_name;
                for (let fn of template.abi.functions){
                    var option = document.createElement("option");
                    option.text = fn.name;
                    ddFunc.add(option);
                }

            } catch (e) {
                setError("Bad template: " + e);
                console.error(e);
            }
            // let y = document.getElementById("template");
            // y.innerHTML = template.template;
        }

        ddFunc.onchange = async function() {
            let fn = template.abi.functions.find(x => x.name === ddFunc.value);
            console.log(fn);
            document.getElementById("argExample").innerText = JSON.stringify(fn.arguments, null, 2);
        }

        document.getElementById("callFunction").onclick = async function() {
            try {
                clearError();
                // todo: submit args
                let submitResponse = await conn.submitFunctionCall(templateAddress, ddFunc.value, true);
                console.log(submitResponse);
                document.getElementById("result").innerText = JSON.stringify(submitResponse, null, 2);
            } catch (e) {
                setError("Bad invoke: " + e);
                console.error(e);
            }
        }

        let btnCallMethod = document.getElementById("callMethod");
        let txtComponentAddress = document.getElementById("componentAddress");
        btnCallMethod.onclick = async function() {
            try {
                clearError();
                // todo: submit args
                let submitResponse = await conn.submitMethodCall(templateAddress, txtComponentAddress.value, ddFunc.value,  true);
                console.log(submitResponse);
                document.getElementById("result").innerText = JSON.stringify(submitResponse, null, 2);
            } catch (e) {
                setError("Bad invoke: " + e);
                console.error(e);
            }
        }
        // console.log(submitResponse);
        // tari.sayHello();
        // window.tari = conn;
    }
    catch(e) {
        console.error(e);
    }
}

main().then(() => console.log("done"));
