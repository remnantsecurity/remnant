FROM rust:1.95.0-slim AS builder

RUN rustup target add x86_64-unknown-linux-musl \
    && apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Build remnant CLI (root workspace: crates/remnant-cli).
COPY rust-toolchain.toml ./
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked --target x86_64-unknown-linux-musl

# Build proxy (standalone workspace: integrations/npm-registry-proxy/).
COPY integrations/npm-registry-proxy/ integrations/npm-registry-proxy/
RUN cargo build --release --locked \
    --manifest-path integrations/npm-registry-proxy/Cargo.toml \
    --target x86_64-unknown-linux-musl

FROM gcr.io/distroless/static-debian12:nonroot@sha256:aef9602f8710ec12bde19d593fed1f76c708531bb7aba205110f1029786ead7b

ENV PATH=/usr/local/bin:/usr/bin:/bin

COPY --from=builder \
    /build/target/x86_64-unknown-linux-musl/release/remnant \
    /usr/local/bin/remnant

COPY --from=builder \
    /build/integrations/npm-registry-proxy/target/x86_64-unknown-linux-musl/release/remnant-npm-registry-proxy \
    /usr/local/bin/remnant-npm-registry-proxy

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/remnant-npm-registry-proxy"]
