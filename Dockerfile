FROM rust:latest as builder

WORKDIR /usr/src/pal-monitor

COPY . .
RUN cargo install --path .

FROM ubuntu:latest
COPY --from=builder /usr/local/cargo/bin/pal-monitor /usr/local/bin/pal-monitor
CMD ["pal-monitor"]
