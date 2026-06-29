# Stage 1: Fetch fonts (BDF sources + pre-built OTB)
FROM alpine:3 AS fonts

RUN apk add --no-cache curl make tar xz unzip

WORKDIR /src
COPY Makefile .
RUN make fonts

# Stage 2: Build Rust binary
FROM rust:1.96-alpine AS builder

RUN apk add --no-cache musl-dev make

WORKDIR /src
COPY . .
COPY --from=fonts /src/fonts/otb/ fonts/otb/

RUN cargo build --release

# Minimal /etc/passwd and /etc/group so the scratch image has a nobody user
RUN printf 'nobody:*:65534:65534:nobody:/_nonexistent:/bin/false\n' > /etc/passwd.min \
 && printf 'nobody:*:65534:\n' > /etc/group.min

# Stage 3: Minimal runtime
FROM scratch

COPY --from=builder /etc/passwd.min /etc/passwd
COPY --from=builder /etc/group.min  /etc/group
COPY --from=builder /src/target/release/tapir /tapir
COPY --from=builder /src/config.toml /config.toml
COPY --from=builder /src/labels/ /labels/
COPY --from=fonts   /src/fonts/otb/ /fonts/otb/

USER nobody

EXPOSE 3000

ENTRYPOINT ["/tapir"]
