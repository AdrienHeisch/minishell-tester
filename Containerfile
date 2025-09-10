FROM rust:alpine

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static
