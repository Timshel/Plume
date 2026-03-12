FROM rust:1.93.1-slim-trixie AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    build-essential \
    gettext \
    libpq-dev \
    libssl-dev \
    openssl \
    pkg-config

WORKDIR /app
COPY . .

RUN ls -l

RUN cargo install wasm-pack
RUN wasm-pack build --target web --release plume-front
RUN cargo build --release --no-default-features --features postgres
RUN cargo build --release --package plume-cli --no-default-features --features postgres

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    libssl3t64

WORKDIR /app

COPY --from=builder /app/po /app/po
COPY --from=builder /app/plume-front/pkg /app/plume-front/pkg
COPY --from=builder /app/static /app/static

COPY --from=builder /app/target/release/plm /bin/
COPY --from=builder /app/target/release/plume /bin/

CMD ["plume"]

EXPOSE 7878
