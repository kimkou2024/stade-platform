# Backend image — multi-stage Rust build.
FROM rust:1-slim AS build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/stade-backend /usr/local/bin/stade-backend
COPY --from=build /app/migrations ./migrations
EXPOSE 8080
ENV BIND_ADDR=0.0.0.0:8080
CMD ["stade-backend"]
