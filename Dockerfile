FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p arda-cli

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 arda
COPY --from=build /src/target/release/arda /usr/local/bin/arda
VOLUME ["/worlds"]
USER arda
ENTRYPOINT ["arda"]
