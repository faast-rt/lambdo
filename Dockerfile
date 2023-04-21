# Step 1: Build
FROM rust:alpine AS build

WORKDIR /app

COPY . .

RUN apk add --no-cache musl-dev && \
  cargo build --release

# Step 2: Run
FROM alpine:latest

ENV RUST_LOG=info

WORKDIR /etc/lambdo

RUN apk add qemu-system-x86_64 libvirt libvirt-daemon dbus polkit qemu-img

COPY --from=build /app/target/release/api /etc/lambdo/lambdo

CMD ["./lambdo"]
