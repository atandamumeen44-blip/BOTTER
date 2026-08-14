FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Force cache bust – ensures Railway uses the latest code
RUN echo "Build timestamp: $(date)"

RUN cargo build --release 2>&1 | tee /tmp/build.log; \
    test -f target/release/sharklet || (echo "BUILD FAILED - dumping log" && cat /tmp/build.log && exit 1)

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/sharklet ./sharklet
COPY --from=builder /app/config ./config

EXPOSE 8080
CMD ["./sharklet"]
