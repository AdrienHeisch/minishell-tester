FROM rust:bookworm

RUN apt-get update
RUN apt-get install -y openssl libssl-dev bubblewrap

WORKDIR /usr/src/maxitest
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src

RUN cargo fetch
RUN cargo build --release
RUN mv target/release/maxitest /bin/maxitest
