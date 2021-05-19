FROM rust:1.52.1

WORKDIR /usr/src/sugarcane

COPY . .

RUN cargo test
