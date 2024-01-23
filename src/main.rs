use clap::Parser as ClapParser;
pub use jsonrpsee::{
    client_transport::ws::{self, EitherStream, Url, WsTransportClientBuilder},
    core::client::{Client, ClientT, SubscriptionClientT},
    rpc_params,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub async fn client(url: &str) -> Result<Client, Box<dyn std::error::Error>> {
    let url = Url::parse(url)?;

    let (sender, receiver) = WsTransportClientBuilder::default().build(url).await?;

    Ok(Client::builder()
        .max_buffer_capacity_per_subscription(4096)
        .build_with_tokio(sender, receiver))
}

#[derive(Debug, ClapParser)]
enum Command {
    Subscribe,
    Storage(StorageOpts),
}

#[derive(Debug, ClapParser)]
struct StorageOpts {
    id: String,
    hash: String,
    key: String,
}

/// The storage item received as parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageQuery {
    /// The provided key.
    pub key: String,
    /// The type of the storage query.
    #[serde(rename = "type")]
    pub query_type: StorageQueryType,
}

/// The type of the storage query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageQueryType {
    /// Fetch the value of the provided key.
    Value,
    /// Fetch the hash of the value of the provided key.
    Hash,
    /// Fetch the closest descendant merkle value.
    ClosestDescendantMerkleValue,
    /// Fetch the values of all descendants of they provided key.
    DescendantsValues,
    /// Fetch the hashes of the values of all descendants of they provided key.
    DescendantsHashes,
}

async fn subscribe() -> Result<(), Box<dyn std::error::Error>> {
    let client = client("ws://localhost:9944").await?;

    let mut subscription = SubscriptionClientT::subscribe::<Box<RawValue>, _>(
        &client,
        "chainHead_unstable_follow",
        rpc_params![false],
        "chainHead_unstable_unfollow",
    )
    .await?;

    println!("Subscription ID: {:?}\n", subscription.kind());

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    while let Some(event) = subscription.next().await {
        let event = event?;

        println!("ChainHead event: {:?}\n", event);
    }

    Ok(())
}

async fn storage(opts: StorageOpts) -> Result<(), Box<dyn std::error::Error>> {
    let client = client("ws://localhost:9944").await?;

    let items = vec![StorageQuery {
        key: opts.key,
        query_type: StorageQueryType::Value,
    }];

    println!("ID: {:?}", opts.id);
    println!("hash: {:?}", opts.hash);
    println!("Storage items: {:?}\n", items);

    let response: Box<RawValue> = client
        .request(
            "chainHead_unstable_storage",
            rpc_params![opts.id, opts.hash, items],
        )
        .await?;

    println!("Storage response: {:?}\n", response);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Command::parse();

    match args {
        Command::Subscribe => subscribe().await,
        Command::Storage(opts) => storage(opts).await,
    }
}
