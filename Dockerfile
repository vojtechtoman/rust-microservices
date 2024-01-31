FROM rust:1.75.0-slim-buster as builder
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes protobuf-compiler
WORKDIR /usr/local/src
COPY . .
RUN cargo build --release
RUN rm -rf target/release/*.*
RUN find target/release -mindepth 1 -maxdepth 1 -type d -print0 | xargs -0 rm -rf

FROM debian:buster-slim as todo-api
COPY --from=builder /usr/local/src/target/release/todo-api /usr/local/bin/app
WORKDIR /usr/local/bin
CMD [ "./app" ]

FROM debian:buster-slim as todo-grpc-service
COPY --from=builder /usr/local/src/target/release/todo-grpc-service /usr/local/bin/app
WORKDIR /usr/local/bin
CMD [ "./app" ]