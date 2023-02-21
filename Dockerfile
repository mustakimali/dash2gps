FROM lukemathwalker/cargo-chef:latest-rust-1.67 AS chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates libtesseract-dev clang \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release
RUN ls -lsah target/release

FROM debian:bullseye-slim AS runtime
WORKDIR app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends libtesseract-dev ffmpeg clang \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/dash2gps /app
COPY eng.traineddata /app

ENTRYPOINT ["/app/dash2gps"]
