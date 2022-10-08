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
        const tari = await import("../pkg/index.js").catch(console.error);
        window.tariLib = tari;
        let conn = new tari.TariConnection("http://localhost:18145/json_rpc", "222f59f59229dda1f3453a143b12eb2947e2217664be1373a85b25d9cb3ab642");
        let id = await conn.getIdentity();
        console.log(id);
        let submitResponse = await conn.submitFunctionCall("D509A78BDBF2DA18733F02223547CDCB089E4E99B391E14454E57EEBD33900CB", "asf");
        // console.log(submitResponse);
        // tari.sayHello();
        // window.tari = conn;
    }
    catch(e) {
        console.error(e);
    }
}

main().then(() => console.log("done"));
