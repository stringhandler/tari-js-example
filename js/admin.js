import axios from "axios";


function connect() {
  let conn = new tari.TariConnection("http://localhost:18200/json_rpc", window.private_key);
  return conn;
}
function onUpdateWalletClick() {
  console.log("clicked");

  let private_key = document.getElementById("txtPrivateKey").value;
  window.privateKey = private_key;
  window.wallet = new Wallet();
}

class Wallet {
  constructor() {
    var self = this;
    self.account_template = "asdf";
      self.account_component = "asfacomp";
  }

}

async function onMintCase() {
  console.log("mint");
  try {
    let conn = connect();
    let tx = {
      instructions: [
        {
          type: "CallMethod",
          template_address: params.t,
          component_address: params.c,
          method: "mint_case",
          args: [
            document.getElementById("txtName").value,
            document.getElementById("txtImageUrl".value)
          ]
        },
        {
          type: "PutLastInstructionOutputOnWorkspace",
          key: "case"
        },
        {
          type: "CallMethod",
          template_address: window.wallet.account_template,
          component_address: window.wallet.account_component,
          method: "deposit",
          args: [
            "Workspace(case)"
          ]
        }
      ]
    }
    await conn.submit(tx);
  }
  catch (e)
  {
    console.log(e);
  }
}

async function main() {
  const params = new Proxy(new URLSearchParams(window.location.search), {
    get: (searchParams, prop) => searchParams.get(prop),
  });
  if (!params.t) {
    alert("You have no set a template address in the query");

  }
  if (!params.c) {
    alert("you have not set a component address in the query");
  }
  window.params = params;
  try {
    const tari = await import("../pkg/index.js").catch(console.error);
    window.tariLib = tari;
  }
  catch (e){
    console.error(e);
  }

  document.getElementById("btnSetWallet").onclick = onUpdateWalletClick;

  //   // let id = await conn.getIdentity();
  //   console.log(id);
  //   let templates = await conn.getTemplates(10);
  //   console.log(templates);
  //   let x = document.getElementById("templates");
  //   for (let template of templates.templates){
  //     let x = document.getElementById("templates");
  //     var option = document.createElement("option");
  //     option.text = template.address;
  //     x.add(option);
  //   }
  //   let hTemplateName =document.getElementById("templateName");
  //   let ddFunc = document.getElementById("func");
  //   x.onchange = async function() {
  //     try {
  //       clearError();
  //       window.templateAddress = x.value;
  //       window.template = await conn.getTemplate(x.value);
  //       console.log(template);
  //       hTemplateName.innerText = template.abi.template_name;
  //       for (let fn of template.abi.functions){
  //         var option = document.createElement("option");
  //         option.text = fn.name;
  //         ddFunc.add(option);
  //       }
  //
  //     } catch (e) {
  //       setError("Bad template: " + e);
  //       console.error(e);
  //     }
  //     // let y = document.getElementById("template");
  //     // y.innerHTML = template.template;
  //   }
  //
  //   ddFunc.onchange = async function() {
  //     let fn = template.abi.functions.find(x => x.name === ddFunc.value);
  //     console.log(fn);
  //     document.getElementById("argExample").innerText = JSON.stringify(fn.arguments, null, 2);
  //   }
  //
  //   document.getElementById("callFunction").onclick = async function() {
  //     try {
  //       clearError();
  //       // todo: submit args
  //       let submitResponse = await conn.submitFunctionCall(templateAddress, ddFunc.value, true);
  //       console.log(submitResponse);
  //       document.getElementById("result").innerText = JSON.stringify(submitResponse, null, 2);
  //     } catch (e) {
  //       setError("Bad invoke: " + e);
  //       console.error(e);
  //     }
  //   }
  //
  //   let btnCallMethod = document.getElementById("callMethod");
  //   let txtComponentAddress = document.getElementById("componentAddress");
  //   btnCallMethod.onclick = async function() {
  //     try {
  //       clearError();
  //       // todo: submit args
  //       let submitResponse = await conn.submitMethodCall(templateAddress, txtComponentAddress.value, ddFunc.value,  true);
  //       console.log(submitResponse);
  //       document.getElementById("result").innerText = JSON.stringify(submitResponse, null, 2);
  //     } catch (e) {
  //       setError("Bad invoke: " + e);
  //       console.error(e);
  //     }
  //   }
  //   // console.log(submitResponse);
  //   // tari.sayHello();
  //   // window.tari = conn;
  // }
  // catch(e) {
  //   console.error(e);
  // }
}

main().then(() => console.log("ready"));
