### install packages
FROM rust:bullseye as deps

# install docker inside the image in order to send shell commands to wireguard container
RUN apt update && \
    apt install -qy curl && \
    curl -sSL https://get.docker.com/ | sh

### build the proxy
FROM deps as builder

WORKDIR /proxy

# copy rust files
COPY . .

# build the proxy
RUN cargo build --release

### run the proxy
FROM debian:bullseye-slim

WORKDIR /proxy

# copy the proxy binary
COPY --from=builder /proxy/target/release/omnia-proxy .

# run the proxy
CMD ["./omnia-proxy"]
