use anyhow::Context;
use pathfinder_lib::{
    cairo,
    config::{self, EthereumConfig},
    ethereum, rpc, sequencer, state,
    storage::Storage,
};
use std::sync::Arc;
use tracing::info;
use web3::{transports::Http, Web3};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let config =
        config::Configuration::parse_cmd_line_and_cfg_file().context("Parsing configuration")?;

    info!("🏁 Starting node.");
    let eth_transport = ethereum_transport(config.ethereum)
        .await
        .context("Creating Ethereum transport")?;

    let network_chain = ethereum::chain(&eth_transport)
        .await
        .context("Determining Ethereum chain")?;

    info!("Identified chain as {}", network_chain);

    let database_path = match network_chain {
        ethereum::Chain::Mainnet => "mainnet.sqlite",
        ethereum::Chain::Goerli => "goerli.sqlite",
    };

    let storage = Storage::migrate(database_path.into()).unwrap();
    info!("Database initialized");
    let sequencer = sequencer::Client::new(network_chain).unwrap();
    let sync_state = Arc::new(state::SyncState::default());

    let _sync_handle = tokio::spawn(state::sync(
        storage.clone(),
        eth_transport,
        network_chain,
        sequencer.clone(),
        sync_state.clone(),
    ));
    info!("Sync process started");

    // TODO: the error could be recovered, but currently it's required for startup. There should
    // not be other reason for the start to fail than python script not firing up.
    let (call_handle, _jh) = cairo::ext_py::start(
        storage.path().into(),
        std::num::NonZeroUsize::new(2).unwrap(),
        futures::future::pending(),
    )
    .await
    .context("Creating python process for call handling. Have you setup and activate the python `VIRTUAL_ENV` in the `py` directory?")?;
    info!("Python sub-processes started");

    let api = rpc::api::RpcApi::new(storage, sequencer, network_chain, sync_state)
        .with_call_handling(call_handle);

    let (_rpc_handle, local_addr) =
        rpc::run_server(config.http_rpc_addr, api).context("Starting the RPC server")?;
    info!("📡 HTTP-RPC server started on: {}", local_addr);
    let () = std::future::pending().await;

    Ok(())
}

/// Creates an [Ethereum transport](Web3<Http>) from the configuration.
///
/// This includes setting:
/// - the [Url](reqwest::Url)
/// - the user-agent (if provided)
/// - the password (if provided)
async fn ethereum_transport(config: EthereumConfig) -> anyhow::Result<Web3<Http>> {
    let client = reqwest::Client::builder();
    let client = match config.user {
        Some(user_agent) => client.user_agent(user_agent),
        None => client,
    }
    .build()
    .context("Creating HTTP client")?;

    let mut url = config.url;
    url.set_password(config.password.as_deref())
        .map_err(|_| anyhow::anyhow!("Setting password"))?;

    let client = Http::with_client(client, url);

    Ok(Web3::new(client))
}
