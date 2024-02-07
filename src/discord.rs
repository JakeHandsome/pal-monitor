use crate::docker::{self, get_palworld_docker_container, ContainerIsRunning};
use anyhow::Result;
use poise::serenity_prelude::{self as serenity, Mention};
use std::time::Duration;
use tracing::info;

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
pub async fn create_client(discord_token: &str) -> Result<serenity::Client> {
    let intents = serenity::GatewayIntents::non_privileged();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                status(),
                start(),
                stop(),
                //    save()
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();
    let client = serenity::ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await?;
    Ok(client)
}

/// Gets the status of the palworld server
#[poise::command(prefix_command, slash_command)]
async fn status(ctx: Context<'_>) -> Result<(), Error> {
    info!("status command called by {}", ctx.author().name);
    ctx.defer().await?;
    let server_running = docker::get_palworld_docker_container(None)
        .await?
        .is_running();
    if server_running {
        if let Ok((players, count)) = docker::get_players().await {
            let memory = docker::get_memory_stats()?;
            ctx.say(format!(
                "```\n{count} players online\n{players}\n---\n{memory}\n```"
            ))
            .await?;
        } else {
            ctx.say("Server is booting/restarting").await?;
        }
    } else {
        ctx.say("Server Offline").await?;
    }
    Ok(())
}
/// Starts server if it is offline
#[poise::command(prefix_command, slash_command)]
async fn start(ctx: Context<'_>) -> Result<(), Error> {
    info!("start command called");
    ctx.defer().await?;
    let container = docker::get_palworld_docker_container(None).await?;
    if container.is_running() {
        ctx.say("Server is already running").await?;
    } else {
        let res = docker::start_docker_container().await;
        match res {
            Ok(_) => {
                info!("Server starting");
                ctx.say("Server is starting").await?;
                while docker::run_rcon_command(vec!["info"]).await.is_err() {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                info!("Server Started");
                ctx.reply(format!("Server started {}", Mention::from(ctx.author().id)))
                    .await?
            }
            Err(e) => ctx.say(format!("Server failed to start {:?}", e)).await?,
        };
    };
    Ok(())
}

/// Saves and stops the server if all players are offline
#[poise::command(prefix_command, slash_command)]
async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    info!("stop command called");
    ctx.defer().await?;
    if let Ok((players, count)) = docker::get_players().await {
        if count != 0 {
            ctx.say(format!("{count} players are still online\n{players}"))
                .await?;
        } else {
            ctx.say("Server shutting down").await?;
            let backup = docker::exec_command(vec!["backup"]).await?;
            ctx.say(backup.to_string()).await?;
            docker::stop_docker_container().await?;
            while docker::run_rcon_command(vec!["info"]).await.is_ok() {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            info!("Server Stopped");
            ctx.reply(format!("Server Stopped {}", Mention::from(ctx.author().id)))
                .await?;
        }
    } else {
        ctx.say("Server is already offline").await?;
    }
    Ok(())
}

/// Saves the game and makes a backup
#[poise::command(prefix_command, slash_command)]
async fn save(ctx: Context<'_>) -> Result<(), Error> {
    info!("save command called");
    ctx.defer().await?;
    if (docker::get_players().await).is_ok() {
        let backup = docker::exec_command(vec!["backup"]).await?;
        ctx.say(format!("backup completed\n{backup}")).await?;
    } else {
        ctx.say("Server is offline").await?;
    }
    Ok(())
}
