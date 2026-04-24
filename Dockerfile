FROM rust:1.77 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/Intelligence_Query .
COPY seed.json .
EXPOSE 3000
CMD ["./Intelligence_Query"]v