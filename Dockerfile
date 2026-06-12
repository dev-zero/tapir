# Build stage
FROM rust:1.87-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM scratch

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/tapir /tapir
COPY --from=builder /src/config.toml /config.toml
COPY --from=builder /src/labels/ /labels/
COPY --from=builder /src/fonts/ /fonts/

EXPOSE 3000

ENTRYPOINT ["/tapir"]
