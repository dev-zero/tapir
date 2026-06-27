# Build stage
FROM rust:1.87-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# Minimal /etc/passwd and /etc/group so the scratch image has a nobody user
RUN printf 'nobody:*:65534:65534:nobody:/_nonexistent:/bin/false\n' > /etc/passwd.min \
 && printf 'nobody:*:65534:\n' > /etc/group.min

# Runtime stage
FROM scratch

COPY --from=builder /etc/passwd.min /etc/passwd
COPY --from=builder /etc/group.min  /etc/group
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/tapir /tapir
COPY --from=builder /src/config.toml /config.toml
COPY --from=builder /src/labels/ /labels/
COPY --from=builder /src/fonts/ /fonts/

USER nobody

EXPOSE 3000

ENTRYPOINT ["/tapir"]
