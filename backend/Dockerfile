FROM rust:1.77-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /app

# Build context is the workspace root.
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/

# Dummy source caches dependency compilation for the backend workspace member.
RUN mkdir -p backend/src && echo "fn main(){}" > backend/src/main.rs
RUN cargo build --release -p runechat-backend && rm -rf backend/src

COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
RUN touch backend/src/main.rs && cargo build --release -p runechat-backend

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/runechat-backend /usr/local/bin/runechat
EXPOSE 3000
CMD ["runechat"]
