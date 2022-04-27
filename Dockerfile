FROM rust:1.52.1

WORKDIR /usr/src/bamboo

COPY . .

RUN cargo test
