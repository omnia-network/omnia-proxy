### install packages
FROM rust:bullseye AS deps
WORKDIR /proxy
# install docker inside the image in order to send shell commands to wireguard container
RUN apt update && \
    apt install -qy curl && \
    curl -sSL https://get.docker.com/ | sh
RUN cargo install cargo-chef

### prepare the build
FROM deps AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

### build the proxy
FROM chef AS builder 
COPY --from=planner /proxy/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin app

### run the proxy
FROM debian:bullseye-slim AS runner
WORKDIR /proxy
# copy the proxy binary
COPY --from=builder /proxy/target/release/omnia-proxy .
# run the proxy
CMD ["./omnia-proxy"]
