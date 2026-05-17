FROM rust:1.95 AS builder
WORKDIR /app
COPY . .
RUN cargo build --bin marmot-server --no-default-features --features server --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libglib2.0-0 libpango1.0-0 libcairo2 libpangocairo-1.0-0
COPY --from=builder /app/target/release/marmot-server /usr/local/bin/
EXPOSE 3000
CMD ["marmot-server"]
