#![allow(dead_code)]
use goose::prelude::*;

async fn syncing(user: &mut GooseUser) -> GooseTaskResult {
    let json = &serde_json::json!({
        "jsonrpc": "2.0",
        "id": "0",
        "method": "starknet_syncing",
    });

    user.post_json("", json).await?;

    Ok(())
}

async fn transaction_hash(user: &mut GooseUser) -> GooseTaskResult {
    let json = &serde_json::json!({
        "jsonrpc": "2.0",
        "id": "0",
        "method": "starknet_getTransactionByHash",
        "params": {
            // "transaction_hash": "deadbeef",
           "transaction_hash": "0x66583c8b196c59f6feb56cf76fd29126525186a056836f2998a76c6c1ef3f45",
        }
    });

    let response = user.post_json("", json).await?;
    // println!(
    //     "Response: {}",
    //     response.response.unwrap().text().await.unwrap()
    // );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_taskset(taskset!("pathfinder").register_task(task!(transaction_hash)))
        .execute()
        .await?
        .print();

    Ok(())
}
