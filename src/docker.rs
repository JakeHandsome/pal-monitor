use anyhow::{bail, Context as _, Result};
use bollard::{
    container::ListContainersOptions,
    exec::{CreateExecOptions, StartExecResults},
    service::ContainerSummary,
    Docker,
};
use futures_util::stream::StreamExt;
use itertools::Itertools;
use serde::ser::Serialize;
use std::{process::Command, str};
use tracing::info;

pub async fn get_palworld_docker_container(docker: Option<&Docker>) -> Result<ContainerSummary> {
    let opt;
    let docker = match docker {
        Some(d) => d,
        None => {
            opt = Docker::connect_with_local_defaults()?;
            &opt
        }
    };
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

pub async fn start_docker_container() -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .start_container::<String>("palworld-server", None)
        .await
        .context("Failed to start container")?;
    Ok(())
}

pub async fn stop_docker_container() -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .stop_container("palworld-server", None)
        .await
        .context("Failed to stop container")?;
    Ok(())
}

pub async fn get_players() -> Result<(String, usize)> {
    let data = run_rcon_command(vec!["ShowPlayers"]).await?;
    let response = data.lines().collect::<Vec<&str>>();
    // First line is csv header, don't count it in character count
    let count = response.len() - 1;
    let players = response.join("\n");
    Ok((players, count))
}

pub fn get_memory_stats() -> Result<String> {
    let output = Command::new("free").arg("-h").output()?.stdout;
    Ok(String::from_utf8(output)?)
}

pub async fn run_rcon_command(mut command: Vec<&str>) -> Result<String> {
    let mut rcon_command = vec!["rcon-cli"];
    rcon_command.append(&mut command);
    let res = exec_command(rcon_command).await?;
    if res.is_empty() {
        bail!("No rcon response");
    } else {
        Ok(res)
    }
}
pub async fn exec_command<T>(command: Vec<T>) -> Result<String>
where
    T: Into<String> + Serialize + Default,
{
    let docker = Docker::connect_with_local_defaults()?;
    let container = get_palworld_docker_container(Some(&docker)).await?;
    info!("{:?}", container.state);
    if container.is_running() {
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

pub trait ContainerIsRunning {
    fn is_running(&self) -> bool;
}

impl ContainerIsRunning for ContainerSummary {
    fn is_running(&self) -> bool {
        match &self.state {
            Some(state) => state.as_str() == "running",
            None => false,
        }
    }
}
