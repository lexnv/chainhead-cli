use std::collections::HashMap;

use clap::Parser as ClapParser;
pub use jsonrpsee::{
    client_transport::ws::{self, EitherStream, Url, WsTransportClientBuilder},
    core::client::{Client, ClientT, SubscriptionClientT},
    rpc_params,
    types::SubscriptionId,
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
    // let client = client("ws://localhost:9944").await?;
    let client = client("wss://rpc.polkadot.io:443").await?;

    let mut subscription = SubscriptionClientT::subscribe::<Box<RawValue>, _>(
        &client,
        "chainHead_v1_follow",
        rpc_params![false],
        "chainHead_v1_unfollow",
    )
    .await?;

    println!("Subscription ID: {:?}\n", subscription.kind());

    let sub_id = match subscription.kind().clone() {
        jsonrpsee::core::client::SubscriptionKind::Subscription(sub) => match sub {
            SubscriptionId::Num(num) => num.to_string(),
            SubscriptionId::Str(str) => str.to_string(),
        },
        jsonrpsee::core::client::SubscriptionKind::Method(_) => todo!(),
        _ => todo!(),
    };
    println!("Sub id {:?}", sub_id);

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    #[derive(Debug, PartialEq, Eq)]
    enum BlockState {
        New,
        Best,
        Finalized,
    }

    let mut blocks = HashMap::new();

    while let Some(event) = subscription.next().await {
        let event = event?;

        #[derive(Serialize, Deserialize, Debug)]
        struct Init {
            event: String,
            finalizedBlockHashes: Vec<String>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct Finalized {
            event: String,
            finalizedBlockHashes: Vec<String>,
            prunedBlockHashes: Vec<String>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct NewBlock {
            event: String,
            blockHash: String,
            parentBlockHash: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct BestBlock {
            event: String,
            bestBlockHash: String,
        }

        if let Ok(result) = serde_json::from_str::<Init>(&event.to_string()) {
            println!("Init event: {:?}\n", result);

            for hash in result.finalizedBlockHashes {
                println!("  Finalized block hash: {:?}\n", hash);

                let response: Box<RawValue> = client
                    .request("chainHead_v1_unpin", rpc_params![sub_id.clone(), hash])
                    .await?;
                println!(" Response for unpinning {}", response);
            }

            continue;
        }

        if let Ok(result) = serde_json::from_str::<NewBlock>(&event.to_string()) {
            println!("NewBlock event: {:?}\n", result);

            match blocks.entry(result.blockHash.clone()) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    panic!(
                        "
                        Block hash already exists in the map.
                        Current state: {:?}
                        New state: {:?}
                        Hash {:?}
                        ",
                        entry.get(),
                        BlockState::New,
                        result.blockHash
                    );
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    println!("  - This is a new entry");
                    entry.insert(BlockState::New);
                }
            }

            blocks.insert(result.blockHash.clone(), BlockState::New);

            continue;
        }

        if let Ok(result) = serde_json::from_str::<BestBlock>(&event.to_string()) {
            println!("BestBlock event: {:?}\n", result);

            match blocks.entry(result.bestBlockHash.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    println!("  - Advancing block to Best state from {:?}", entry.get());
                    if entry.get() != &BlockState::New {
                        panic!(
                            "
                            (best occupied) Expected block entry occupied with New state
                            Current state: {:?}
                            Hash {:?}
                            ",
                            entry.get(),
                            result.bestBlockHash
                        );
                    }

                    *entry.get_mut() = BlockState::Best;
                }
                std::collections::hash_map::Entry::Vacant(_entry) => {
                    panic!(
                        "
                        (best vacant) Expected block entry occupied with New state
                        "
                    );
                }
            }

            continue;
        }

        if let Ok(result) = serde_json::from_str::<Finalized>(&event.to_string()) {
            println!("Finalized event: {:?}\n", result);

            for hash in result
                .finalizedBlockHashes
                .iter()
                .chain(result.prunedBlockHashes.iter())
            {
                match blocks.entry(hash.clone()) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        println!(" - Finalizing block from {:?}", entry.get());
                        if entry.get() != &BlockState::New || entry.get() != &BlockState::Best {
                            panic!(
                                "
                                (finalized occupied) Expected block entry occupied with New or Best state
                                Current state: {:?}
                                Hash: {:?}
                                ",
                                entry.get(),
                                hash
                            );
                        }

                        *entry.get_mut() = BlockState::Finalized;
                    }
                    std::collections::hash_map::Entry::Vacant(_entry) => {
                        panic!(
                            "
                            (finalized vacant) Expected block entry for {:?}
                            ",
                            hash,
                        );
                    }
                }

                println!("  Unpining block hash: {:?}\n", hash);

                let response: Box<RawValue> = client
                    .request("chainHead_v1_unpin", rpc_params![sub_id.clone(), hash])
                    .await?;
                println!(" Response for unpinning {}", response);
            }

            continue;
        }

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
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("setting default subscriber failed");

    let args = Command::parse();

    match args {
        Command::Subscribe => subscribe().await,
        Command::Storage(opts) => storage(opts).await,
    }
}
