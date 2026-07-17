FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY src src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --create-home --uid 10001 vrcdog
WORKDIR /app
COPY --from=builder /app/target/release/vrcdog-server /usr/local/bin/vrcdog-server
RUN mkdir -p /app/data && chown -R vrcdog:vrcdog /app
USER vrcdog
ENV VRCDOG_HOST=0.0.0.0
ENV VRCDOG_PORT=11451
ENV VRCDOG_DATA_FILE=/app/data/server-state.json
EXPOSE 11451
VOLUME ["/app/data"]
ENTRYPOINT ["vrcdog-server"]
