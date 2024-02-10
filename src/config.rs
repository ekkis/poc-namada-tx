#[derive(clap::Parser, Debug)]
pub struct AppConfig {
    #[clap(long, env)]
    pub private_key: String,

    #[clap(long, env)]
    pub chain_id: String,

    #[clap(long, env)]
    pub rpc: String,

    #[clap(long, env)]
    pub token: String,

    #[clap(long, env)]
    pub target: String,

    #[clap(long, env)]
    pub amount: u64,
}
