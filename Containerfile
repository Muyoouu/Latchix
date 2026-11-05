# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.99.1-alpine AS builder

RUN apk add --no-cache cmake gcc make musl-dev nasm perl

WORKDIR /src

COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml clippy.toml ./
COPY crates ./crates
COPY xtask ./xtask

RUN cargo build --locked --release --target x86_64-unknown-linux-musl -p latchix-cli

FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/latchix /usr/local/bin/latchix

USER 65532:65532

EXPOSE 2222 8080
STOPSIGNAL SIGTERM

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/latchix", "health", "--config", "/etc/latchix/config.toml"]

ENTRYPOINT ["/usr/local/bin/latchix"]
CMD ["serve", "--config", "/etc/latchix/config.toml"]

LABEL org.opencontainers.image.title="Latchix"
LABEL org.opencontainers.image.description="API-first SFTP server with hard multi-tenancy"
LABEL org.opencontainers.image.licenses="Apache-2.0"
