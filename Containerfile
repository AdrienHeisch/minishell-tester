FROM rust:latest

RUN apt-get update
RUN apt-get install -y openssl libssl-dev bubblewrap

WORKDIR /usr/src/maxitest

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN mkdir src
RUN echo 'fn main() {}' > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm target/release/maxitest* target/release/deps/maxitest*
RUN rm -rf src

COPY src src
RUN cargo build --release
RUN mv target/release/maxitest /bin/maxitest
