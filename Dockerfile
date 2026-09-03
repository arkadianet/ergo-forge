# ergo-web in a container: build the release binary, ship it with the UI.
# Stateless, no outbound calls; configure with BIND_ADDR / RUST_LOG.

FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
# The engine crates are git dependencies pinned in Cargo.toml; the lock file
# is committed, so this is a reproducible fetch.
RUN cargo build --release -p ergo-web --locked

FROM debian:bookworm-slim
RUN useradd --system --no-create-home ergo
WORKDIR /app
COPY --from=build /src/target/release/ergo-web /usr/local/bin/ergo-web
COPY ui /app/ui
COPY examples /app/examples
USER ergo
# Public-instance defaults: rate-limit the engine routes; set EXPLORER_URL to
# enable chain lookups; RATE_LIMIT_PER_MINUTE=0 turns limiting off.
ENV BIND_ADDR=0.0.0.0:8080 UI_DIR=/app/ui EXAMPLES_DIR=/app/examples/contracts RUST_LOG=info RATE_LIMIT_PER_MINUTE=120
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["/bin/sh", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080 && printf 'GET /api/v1/health HTTP/1.0\r\n\r\n' >&3 && grep -q '\"ok\"' <&3"]
ENTRYPOINT ["/usr/local/bin/ergo-web"]
