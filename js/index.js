import axios from "axios";



function toJsonRpc(method, params){
    return {
        jsonrpc: '2.0',
        method,
        params,
        id: 1,
    }
}


class TariConnectionJs {

    // private kv: KeyRing;
    constructor(url) {
        this.url = url
        // init

        // this.kv = KeyRing.new();
    }

    async getIdentity() {
        return (await axios({
            method: 'post',
            url: this.url,
            data: toJsonRpc("get_identity", []),

        })).data.result
    }
}

async function main() {
    try {
        const wasm = await import("../pkg/index.js").catch(console.error);
        window.wasm = wasm;
        let conn = new wasm.TariConnection("http://localhost:18145/json_rpc");
        let id = await conn.getIdentity();
        console.log(id);
        let submitResponse = await conn.submitFunctionCall("asdf", "asf");
        console.log(submitResponse);
        wasm.sayHello();
    }
    catch(e) {
        console.error(e);
    }
}

main().then(() => console.log("done"));
