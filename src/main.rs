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
	args::{InputAmount, TxBuilder},
	signing::default_sign,
	tendermint::abci::Code,
	core::types::{
		chain::ChainId,
		address::Address,
		masp::{TransferSource, TransferTarget},
		key::{common::SecretKey, RefTo},
		transaction::ResultCode
	},
};

#[tokio::main]
async fn main() {
	dotenv().ok();
	let config = Arc::new(AppConfig::parse());
	let url = Url::from_str(&config.rpc).expect("invalid RPC address");
	let http_client = HttpClient::new(url).unwrap();
	let shielded_ctx = FsShieldedUtils::new("masp".into());
	let wallet = FsWalletUtils::new("wallet".into());

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

	let mut wallet = sdk.wallet.write().await;
	let kp = wallet.insert_keypair(
			"donor".to_string(),
			false,
			sk.clone(),
			None,
			Some(source.clone()),
			None,
		)
		.unwrap();
	println!("insert_keypair={:?}", kp);
	
	let keys = &wallet.get_secret_keys();
	println!("keys={:?}", keys);
	println!("balance={:?}", token::read_balance(&wallet, token, source).await);
	drop(wallet);

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
