FROM rust:bookworm

RUN apt-get update
RUN apt-get install -y openssl libssl-dev bubblewrap
