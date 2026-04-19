FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /app

# Build context is the workspace root.
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/

# Dummy source caches dependency compilation for the backend workspace member.
# We also need a dummy frontend/src-tauri because it's a workspace member;
# Cargo resolves the full workspace before building any crate.
RUN mkdir -p backend/src && echo "fn main(){}" > backend/src/main.rs
RUN mkdir -p frontend/src-tauri/src && \
    echo '[package]\nname = "runechat"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\nname = "runechat_lib"\ncrate-type = ["staticlib", "cdylib", "rlib"]' \
    > frontend/src-tauri/Cargo.toml && \
    echo 'pub fn run() {}' > frontend/src-tauri/src/lib.rs
RUN cargo build --release -p runechat-backend && rm -rf backend/src

COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
# Integration tests live in backend/tests/. They require a live Postgres and run from
# the host checkout via `cargo test` against the compose DB — not during this build.
# Copying them here allows `docker build --target builder && cargo test` in CI later.
COPY backend/tests ./backend/tests
RUN touch backend/src/main.rs && cargo build --release -p runechat-backend

FROM alpine:3.23
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/runechat-backend /usr/local/bin/runechat
EXPOSE 3000
CMD ["runechat"]
