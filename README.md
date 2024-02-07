# Pal-monitor

This container is meant to run alongside `https://github.com/thijsvanloef/palworld-server-docker` and allowed starting
and stopping the server though a discord bot.


```yaml
services:
    pal-monitor:
        image: todo # TODO
        container_name: pal-monitor
        environment:
         - DISCORD_TOKEN=YourTokenHere
        -volumes:
         - /var/run/docker.sock:/var/run/docker.sock # Pass docker socket into container to it can control the other container
```
