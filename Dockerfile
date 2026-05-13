# Stage 1: Build frontend
FROM node:22-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
FROM ubuntu:24.04 AS rust-build
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates \
    cmake g++ make pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Cache dependency build
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Build real binary
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

# Stage 3: Runtime image
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-build /app/target/release/koram ./
COPY --from=frontend-build /app/frontend/dist/ ./frontend/dist/

RUN mkdir -p config cache

EXPOSE 5001
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fs http://localhost:5001/api/health || exit 1
CMD ["./koram"]
