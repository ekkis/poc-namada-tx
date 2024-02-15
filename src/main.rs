use dotenvy::dotenv;
use std::{sync::Arc, str::FromStr};
use clap::Parser;
use namada_poc::{config::AppConfig};
use tendermint_rpc::{HttpClient, Url};
use namada_sdk::{
	rpc,
	NamadaImpl, 
	Namada,
	wallet::fs::FsWalletUtils, 
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
		key::{common::SecretKey, RefTo}
	}
};

#[tokio::main]
async fn main() {
	dotenv().ok();
	let config = Arc::new(AppConfig::parse());
	let url = Url::from_str(&config.rpc).expect("invalid RPC address");
	let http_client = HttpClient::new(url).unwrap();
	let shielded_ctx = FsShieldedUtils::new("masp".into());
	// let wallet = FsWalletUtils::new("wallet".into());
	let dir = "/Users/ekkis/Library/Application Support/Namada/local.6615cacd5792a2c8d73c537b/";
	let mut wallet = FsWalletUtils::new(dir.into());
	wallet.load().expect("Failed to load wallet");
	
	// println!("{:?}", &wallet);

	let token = Address::decode(config.token.clone());
	let token = if let Ok(address) = token {
        address
    } else {
        panic!("Invalid token address");
    };

	let sk = SecretKey::from_str(&config.private_key)
		.expect("Should be able to decode secret key.");
	let source = Address::from(&sk.ref_to());
	// print!("{} / {}", sk, source);

	let bal = rpc::get_token_balance(&http_client, &token, &source).await;
	println!("bal (nam)={:?}", bal);

	let target = Address::decode(config.target.clone()	);
	let target = if let Ok(address) = target {	// WTF is this?
		address
	} else {
		panic!("Invalid target address")
	};
	let sdk = NamadaImpl::new(http_client, wallet, shielded_ctx, NullIo)
		.await
		.expect("unable to initialize Namada context")
		.chain_id(ChainId::from_str(&config.chain_id).unwrap());

	let skfw = sdk.wallet.read().await.get_secret_keys()["donor"];
	println!("{:?}", Address::from(&skfw.into()));
	drop(sdk.wallet.write().await);

	let native_token = rpc::query_native_token(sdk.client()).await.unwrap();
	println!("native_token={:?}", native_token);

	let amt = rpc::denominate_amount(
        sdk.client(),
        sdk.io(),
        &token,
        config.amount.into(),
    )
    .await;
	println!("amt={:?}", amt);

	let mut transfer_tx_builder = sdk.new_transfer(	
        TransferSource::Address(source),
        TransferTarget::Address(target.clone()),
        token.clone(),
        InputAmount::Unvalidated(amt),
    );
	transfer_tx_builder.tx.memo = Some("Test transfer".to_string().as_bytes().to_vec());

	println!("builder={:?}", transfer_tx_builder);

	let (mut transfer_tx, signing_data, _epoch) = transfer_tx_builder
        .build(&sdk)
        .await
        .expect("unable to build transfer");
	println!("tx={:?}, epoch={:?}", transfer_tx, _epoch);
	println!("owner={:?}", signing_data.owner);
	println!("public_keys={:?}", signing_data.public_keys);

    let signed = sdk.sign(
            &mut transfer_tx,
            &transfer_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign reveal pk tx");
	println!("signed={:?}", signed);

    let process_tx_response = sdk.submit(transfer_tx, &transfer_tx_builder.tx).await;
	println!("response={:?}", process_tx_response);

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
