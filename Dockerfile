# syntax=docker/dockerfile:1.7
#
# arda-server + arda CLI. Two variants from one file:
#   docker build -t arda:local .                                  # placeholder library only
#   docker build -t arda:local-ai \
#       --build-context artlib=/abs/path/to/out/art-gen/lib .     # AI library stacked on top
# The AI library is never required and the published image does not carry it.

# Fallback for the optional named context: an empty tree, replaced by
# `--build-context artlib=<dir>` when given.
FROM scratch AS artlib

# Pinned image and toolchain: rust-toolchain.toml says "stable", which would
# otherwise move under a pinned image.
FROM rust:1.98.1-bookworm AS build
ENV RUSTUP_TOOLCHAIN=1.98.1
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p arda-cli -p arda-server \
    && install -D target/release/arda target/release/arda-server -t /out/bin

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
ARG VERSION=dev
ARG REVISION=unknown
LABEL org.opencontainers.image.source="https://github.com/DiceMasterIO/arda" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.title="arda"
RUN groupadd --gid 10001 arda && useradd --uid 10001 --gid 10001 --create-home arda
COPY --from=build /out/bin/arda /out/bin/arda-server /usr/local/bin/
COPY tools/docker-entrypoint.sh /usr/local/bin/arda-entrypoint
COPY assets/tactical/placeholder /opt/arda/library/placeholder
COPY --from=artlib / /opt/arda/library/ai
WORKDIR /opt/arda
VOLUME ["/worlds"]
EXPOSE 8787
USER arda
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
    CMD ["arda-server", "--probe"]
ENTRYPOINT ["arda-entrypoint"]
