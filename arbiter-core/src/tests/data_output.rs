use std::{fs, path::Path};

use super::*;
use crate::{data_collection::{EventLogger, OutputFileType}, middleware::errors::RevmMiddlewareError};

#[tokio::test]
async fn data_capture() {
    let (env, client) = startup_user_controlled().unwrap();
    let (arbx, arby, lex) = deploy_liquid_exchange(client.clone()).await.unwrap();
    println!("Deployed contracts");

    let listener = EventLogger::builder()
        .add(arbx.events(), "arbx")
        .add(arby.events(), "arby")
        .add(lex.events(), "lex");

    listener.run().unwrap();

    for _ in 0..5 {
        arbx.approve(client.address(), U256::from(1))
            .send()
            .await
            .unwrap()
            .await
            .unwrap();
        arby.approve(client.address(), U256::from(1))
            .send()
            .await
            .unwrap()
            .await
            .unwrap();
        lex.set_price(U256::from(10u128.pow(18)))
            .send()
            .await
            .unwrap()
            .await
            .unwrap();
    }

    let _ = env.stop();
    // std::fs::remove_dir_all("./data").unwrap();
}

#[tokio::test]
async fn csv_output() {
    let (env, client) = startup_user_controlled().unwrap();
    let (arbx, arby, lex) = deploy_liquid_exchange(client.clone()).await.unwrap();

    EventLogger::builder()
        .add(arbx.events(), "arbx")
        .add(arby.events(), "arby")
        .add(lex.events(), "lex")
        .file_type(OutputFileType::CSV)
        .run()
        .unwrap();

    // Perform some operations that generate events...
    generate_events(arbx, arby, lex, client.clone()).await.unwrap_or_else(|e| {
        panic!("Error generating events: {}", e);
    });

    let _ = env.stop();
    assert!(Path::new("./data/output.csv").exists());
    // Check if the CSV file was created
}

#[tokio::test]
async fn parquet_output() {
    let (env, client) = startup_user_controlled().unwrap();
    let (arbx, arby, lex) = deploy_liquid_exchange(client.clone()).await.unwrap();

    EventLogger::builder()
        .add(arbx.events(), "arbx")
        .add(arby.events(), "arby")
        .add(lex.events(), "lex")
        .file_type(OutputFileType::Parquet)
        .run()
        .unwrap();

    // Perform some operations that generate events...
    generate_events(arbx, arby, lex, client.clone()).await.unwrap_or_else(|e| {
        panic!("Error generating events: {}", e);
    });

    let _ = env.stop();

    assert!(Path::new("./data/output.parquet").exists());
}

async fn generate_events(arbx: ArbiterToken<RevmMiddleware>, arby: ArbiterToken<RevmMiddleware>, lex: LiquidExchange<RevmMiddleware>, client: Arc<RevmMiddleware>) -> Result<(), RevmMiddlewareError>{
    for _ in 0..5 {
        arbx.approve(client.address(), U256::from(1))
            .send()
            .await
            .unwrap()
            .await?;
        arby.approve(client.address(), U256::from(1))
            .send()
            .await
            .unwrap()
            .await?;
        lex.set_price(U256::from(10u128.pow(18)))
            .send()
            .await
            .unwrap()
            .await?;
    }
    Ok(())
}