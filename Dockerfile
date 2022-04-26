FROM rust:latest as build
RUN USER=root cargo new --bin /opener-service/docker-build
WORKDIR /opener-service/docker-build
#COPY ./Cargo.lock ./
#COPY ./Cargo.toml ./
#RUN cargo install --path . --locked
COPY ./ ./
RUN cargo install --path . --locked

FROM ubuntu:latest
COPY --from=build /usr/local/cargo/bin/opener-service /usr/local/bin/opener-service
ENTRYPOINT ["/usr/local/bin/opener-service"]
