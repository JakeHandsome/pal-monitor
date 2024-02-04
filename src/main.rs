use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use itertools::Itertools;
use serde::ser::Serialize;
use std::{str, time::Duration};
use tracing::{debug, error, info, span, warn, Level};

use anyhow::{bail, Context as _, Result};
use bollard::{
    container::ListContainersOptions,
    exec::{CreateExecOptions, StartExecResults},
    image::ListImagesOptions,
    service::ContainerSummary,
    Docker,
};
#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    start_docker_container().await?;
    loop {
        let out = run_rcon_command(vec!["info"]).await;
        println!("{out:?}");
        tokio::time::sleep(Duration::from_millis(1)).await;
        if out.is_ok() {
            stop_docker_container().await?;
        }
    }
}

async fn get_palworld_docker_container(docker: &Docker) -> Result<ContainerSummary> {
    let container = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?
        .into_iter()
        .filter(|c| match (&c.image, &c.names) {
            (Some(image_name), Some(names)) => {
                image_name == "thijsvanloef/palworld-server-docker:latest"
                    && names.iter().any(|name| name == "/palworld-server")
            }
            _ => false,
        })
        .at_most_one()?
        .context("No container found")?;
    Ok(container)
}

async fn start_docker_container() -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .start_container::<String>("palworld-server", None)
        .await
        .context("Failed to start container")?;
    Ok(())
}

async fn stop_docker_container() -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .stop_container("palworld-server", None)
        .await
        .context("Failed to stop container")?;
    Ok(())
}

async fn run_rcon_command(mut command: Vec<&str>) -> Result<String> {
    let mut rcon_command = vec!["rcon-cli"];
    rcon_command.append(&mut command);
    let res = exec_command(rcon_command).await?;
    if res.is_empty() {
        bail!("No rcon response");
    } else {
        Ok(res)
    }
}
async fn exec_command<T>(command: Vec<T>) -> Result<String>
where
    T: Into<String> + Serialize + Default,
{
    let docker = Docker::connect_with_local_defaults()?;
    let container = get_palworld_docker_container(&docker).await?;
    info!("{:?}", container.state);
    let running = match container.state {
        Some(state) => &state == "running",
        None => false,
    };
    if running {
        let id = container.id.unwrap();
        let exec = docker
            .create_exec(
                &id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    cmd: Some(command),
                    ..Default::default()
                },
            )
            .await?
            .id;
        let mut out = String::new();
        if let StartExecResults::Attached { mut output, .. } =
            docker.start_exec(&exec, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                out.push_str(str::from_utf8(msg.as_ref())?);
            }
        } else {
            unreachable!();
        }
        Ok(out)
    } else {
        bail!("Container not running")
    }
}
