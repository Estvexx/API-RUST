#stage 1
FROM rust:1.91 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release


#stage 2
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/axumlive .
EXPOSE 8001
CMD ["./axumlive"]
