FROM rust:1.81
RUN apt-get update && apt-get install -y iptables
WORKDIR /app
COPY ./p2p/tests/entrypoint.sh entrypoint.sh

COPY . .

RUN cargo build --bin p2p_test
