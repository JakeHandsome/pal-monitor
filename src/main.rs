use anyhow::{Context as _, Result};
use std::{env, time::Duration};
use tracing::info;
mod discord;
mod docker;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let discord_token = env::var("DISCORD_TOKEN").context("'DISCORD_TOKEN' not found")?;
    let mut client = discord::create_client(&discord_token).await?;
    client.start().await.unwrap();
    loop {
        let (players, count) = docker::get_players().await?;
        info!("{count} players online\n{players}");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
