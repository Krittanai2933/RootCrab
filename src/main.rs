#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use bitcoincore_rpc::bitcoincore_rpc_json::{CreateRawTransactionInput, FundRawTransactionOptions};
use bitcoincore_rpc::bitcoin::{Amount, Txid};

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : 100 }]), // recipient address
        json!(null),            // conf target
        json!(null),            // estimate mode
        json!(null),            // fee rate in sats/vb
        json!(null),            // Empty option object
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }
    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn list_wallet_dir(client: &Client) -> bitcoincore_rpc::Result<Vec<String>> {
    #[derive(Deserialize)]
    struct Name {
        name: String,
    }
    #[derive(Deserialize)]
    struct CallResult {
        wallets: Vec<Name>,
    }

    let result: CallResult = client.call("listwalletdir", &[])?;
    Ok(result.wallets.into_iter().map(|n| n.name).collect())
}

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Check Connection
    let info = rpc.get_blockchain_info()?;
    println!("{:?}", info);

    // Create or load the wallet
    let create_result = rpc.create_wallet("testwallet", None, None, None, None)?;
    println!("Created wallet: {:?}", create_result);
    let test_wallet_info = rpc.get_wallet_info()?;
    println!("testwallet info: {:?}", test_wallet_info);

    // Generate a new address
    let address_info = rpc.get_new_address(None, None)?;
    let address = address_info.assume_checked();
    println!("Addr info: {:?}", address);
    
    // Mine 101 blocks to the new address to activate the wallet with mined coins
    let mine_block = rpc.generate_to_address(103, &address)?;
    
    // Prepare a transaction to send 100 BTC
    let utxos: Value = rpc.call("listunspent", &[])?; // Pass an empty slice
    let selected_utxos: Vec<Value> = utxos.as_array()
        .expect("listunspent did not return an array")
        .iter()
        .take(3)
        .map(|u| {
            json!({
                "txid": u.get("txid").expect("Missing txid"),
                "vout": u.get("vout").expect("Missing vout")
            })
        })
        .collect();

    let outputs = json!({
        "bcrt1qq2yshcmzdlznnpxx258xswqlmqcxjs4dssfxt2": 100.0,
        "data": "57652061726520616c6c205361746f7368692121"
    });

    let raw_tx: String = rpc.call("createrawtransaction", &[json!(selected_utxos), outputs.clone()])?;

    let fund_tx: Value = rpc.call("fundrawtransaction", &[json!(raw_tx), json!({"fee_rate": 21})])?;
    let funded_tx_hex = fund_tx
        .get("hex")
        .expect("Missing hex in funded transaction")
        .as_str()
        .expect("hex is not a string");

    let sign_tx: Value = rpc.call("signrawtransactionwithwallet", &[json!(funded_tx_hex)])?;
    let signed_tx_hex = sign_tx
        .get("hex")
        .expect("Missing hex in signed transaction")
        .as_str()
        .expect("hex is not a string");
    // Send the transaction
    let txid: String = rpc.call("sendrawtransaction", &[json!(signed_tx_hex)])?;
    // Write the txid to out.txt
    let mut file = File::create("out.txt")?;
    writeln!(file, "{}", txid)?;
    
    println!("Transaction ID: {}", txid);

    Ok(())
}
