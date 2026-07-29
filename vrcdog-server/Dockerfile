FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY src src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 vrcdog
WORKDIR /app
COPY --from=builder /app/target/release/vrcdog-server /usr/local/bin/vrcdog-server
RUN mkdir -p /app/data && chown -R vrcdog:vrcdog /app
USER vrcdog
ENV VRCDOG_HOST=0.0.0.0
ENV VRCDOG_PORT=11451
ENV VRCDOG_DATA_FILE=/app/data/server-state.json
EXPOSE 11451
VOLUME ["/app/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:11451/ping >/dev/null || exit 1
ENTRYPOINT ["vrcdog-server"]
