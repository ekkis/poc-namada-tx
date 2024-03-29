use dotenvy::dotenv;
use std::{sync::Arc, str::FromStr};
use clap::Parser;
use namada_poc::config::AppConfig;
use tendermint_rpc::{HttpClient, Url};
use namada_sdk::{
	rpc,
	NamadaImpl, 
	Namada,
	wallet::{Wallet, fs::FsWalletUtils}, 
	masp::fs::FsShieldedUtils,
	io::NullIo,
	tx::data::ResultCode,
	args::{InputAmount, TxBuilder},
	signing::default_sign,
	tendermint::abci::Code,
	types::{
		chain::ChainId,
		address::Address,
		masp::{TransferSource, TransferTarget},
		key::RefTo
	}
};

fn get_address(w: &Wallet<FsWalletUtils>, val: &String) -> Address {
	let s = w.find_address(val).map(|addr| addr.to_string()).unwrap();
	Address::decode(s).unwrap()
}

#[tokio::main]
async fn main() {
	dotenv().ok(); // read environment file
	let config = Arc::new(AppConfig::parse());
	let url = Url::from_str(&config.rpc).expect("invalid RPC address");
	let http_client = HttpClient::new(url).unwrap();
	
	// load wallet created in the CLI
	
	let basedir = "Library/Application Support/Namada";
	let basedir = format!("{}/{}/{}", std::env::var("HOME").unwrap(), basedir, &config.chain_id);
	let mut wallet = FsWalletUtils::new(basedir.into());
	wallet.load().expect("Failed to load wallet");
	
	// create a shielded context for our transactions

	let shielded_ctx = FsShieldedUtils::new("masp".into());
	
	// grab a handle to the SDK

	let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, NullIo)
		.await
		.expect("unable to initialize Namada context")
		.chain_id(ChainId::from_str(&config.chain_id).unwrap());

	drop(sdk.wallet.write().await);

	// get addresses for the NAM token and charity account
	
	let token = get_address(&wallet, &config.token);
	let target = get_address(&wallet, &config.target);

	// for the donor account we need the spending key

	let sk = wallet.find_secret_key(config.source.clone(), None)
		.expect("Unable to find key");
	let source = Address::from(&sk.ref_to());

	// create a complete object that reflects the token and amount

	let amt = rpc::denominate_amount(
        sdk.client(),
        sdk.io(),
        &token,
        config.amount.into(),
    )
    .await;

	// create a transaction builder

	let mut xfer = sdk.new_transfer(	
        TransferSource::Address(source),
        TransferTarget::Address(target.clone()),
        token.clone(),
        InputAmount::Unvalidated(amt),
    );
	let memo = String::from("{\"deliver-to\": \"101 Main Street, Lalaland, CA 91002\"}");
	xfer.tx.memo = Some(memo.as_bytes().to_vec());

	let (mut transfer_tx, signing_data, _epoch) = xfer
        .build(&sdk)
        .await
        .expect("unable to build transfer");

	// sign the transaction using the donor key

    sdk.sign(
		&mut transfer_tx,
		&xfer.tx,
		signing_data,
		default_sign,
		(),
	)
	.await
	.expect("unable to sign reveal pk tx");

	// broadcast the transaction to the network

    let process_tx_response = sdk.submit(transfer_tx, &xfer.tx).await;
	// println!("response={:?}", process_tx_response);

	// process the result

	let (sent, tx_hash) = if let Ok(response) = process_tx_response {
        match response {
            namada_sdk::tx::ProcessTxResponse::Applied(r) => (r.code.eq(&ResultCode::Ok), Some(r.hash)),
            namada_sdk::tx::ProcessTxResponse::Broadcast(r) => {
                (r.code.eq(&Code::Ok), Some(r.hash.to_string()))
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    };

	// print the transaction hash on the console for user
	
	println!("sent: {}", sent);
	println!("tx: {}", tx_hash.unwrap());	
}
