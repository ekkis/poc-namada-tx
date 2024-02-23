use dotenvy::dotenv;
use std::{sync::Arc, str::FromStr};
use clap::Parser;
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
		masp::{TransferSource, TransferTarget, PaymentAddress},
	}
};
use namada_poc::config::AppConfig;

fn get_address(w: &Wallet<FsWalletUtils>, val: &String) -> Address {
	let s = w.find_address(val).map(|addr| addr.to_string()).unwrap();
	Address::decode(s).unwrap()
}

fn get_shielded_addr(w: &Wallet<FsWalletUtils>, val: &String) -> PaymentAddress {
	let s = w.find_payment_addr(val).map(|addr| addr.to_string()).unwrap();
	PaymentAddress::from_str(&*s).unwrap()
}

#[tokio::main]
async fn main() {
	dotenv().ok(); // read environment file
	let fee_payer = std::env::var("FEEPAYER").unwrap();

	let config = Arc::new(AppConfig::parse());
	let url = Url::from_str(&config.rpc).expect("invalid RPC address");
	let http_client = HttpClient::new(url).unwrap();
	
	// load wallet created in the CLI
	
	let basedir = "Library/Application Support/Namada";
	let basedir = format!("{}/{}/{}", std::env::var("HOME").unwrap(), basedir, &config.chain_id);
	let mut wallet = FsWalletUtils::new(basedir.into());
	wallet.load().expect("Failed to load wallet");

	// get addresses for the NAM token and merchant account
	
	let token = get_address(&wallet, &config.token);
	let target = get_shielded_addr(&wallet, &config.target);

	// for the customer account we need the spending key
	
	let sk = wallet.find_spending_key(&config.source, None)
		.expect("Unable to find key");

	// and let's get the public key for the fee payer account
	// to allow us to pay for transactions
	
	let pk = wallet.find_public_key(&fee_payer);

	// create a shielded context for our transactions

	let shielded_ctx = FsShieldedUtils::new("masp".into());
	
	// initialise the SDK object

	let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, NullIo)
		.await
		.expect("unable to initialize Namada context")
		.chain_id(ChainId::from_str(&config.chain_id).unwrap());

	// construct a proper amount object

	let amt = rpc::denominate_amount(
        sdk.client(),
        sdk.io(),
        &token,
        config.amount.into(),
    )
    .await;

	// and create a transfer builder

	let mut xfer = sdk.new_transfer(
		TransferSource::ExtendedSpendingKey(sk),
        TransferTarget::PaymentAddress(target),
        token.clone(),
        InputAmount::Unvalidated(amt),
	);

	if pk.is_ok() {
		xfer.clone().signing_keys(vec![pk.unwrap()]);
	} else {
		xfer.tx.disposable_signing_key = true;
		xfer.tx.fee_unshield = Some(TransferSource::ExtendedSpendingKey(sk));
	}
	
	let memo = String::from("{\"deliver-to\": \"101 Main Street, Lalaland, CA 91002\"}");
	xfer.tx.memo = Some(memo.as_bytes().to_vec());

	let (mut transfer_tx, signing_data, _epoch) = xfer
        .build(&sdk)
        .await
        .expect("unable to build transfer");

    sdk.sign(
		&mut transfer_tx,
		&xfer.tx,
		signing_data,
		default_sign,
		(),
	)
	.await
	.expect("unable to sign reveal pk tx");

    let process_tx_response = sdk.submit(transfer_tx, &xfer.tx).await;
	// println!("response={:?}", process_tx_response);

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

	println!("sent: {}", sent);
	println!("tx: {}", tx_hash.unwrap());	
}
