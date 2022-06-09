FROM rust:1.60 as builder

ARG BUILD_OPTS=""

COPY . /atc
WORKDIR /atc

RUN apt-get update && \
    apt-get install -y pkg-config build-essential && \
    cargo build --release --locked $BUILD_OPTS

# --------------------------

FROM ubuntu:latest

COPY --from=builder /atc/target/release/atc /usr/bin/atc

EXPOSE 8080

CMD ["atc"]
