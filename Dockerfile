### install packages
FROM rust:bullseye AS deps
WORKDIR /proxy
# this takes a while due to crates index update, so we do it first
RUN cargo install cargo-chef

### prepare the build
FROM deps AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

### build the proxy
FROM deps AS builder 
COPY --from=planner /proxy/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build

### install linux packages
FROM deps AS installer
# install docker inside the image in order to send shell commands to wireguard container
RUN apt update && \
    apt install -qy curl net-tools iproute2 && \
    curl -sSL https://get.docker.com/ | sh

### run the proxy
FROM installer AS runner
WORKDIR /proxy
# copy the proxy binary
COPY --from=builder /proxy/target/debug/omnia-proxy .

COPY ./scripts/start-proxy.sh .
RUN chmod +x start-proxy.sh

EXPOSE 8081

# run the proxy
CMD ["./start-proxy.sh"]
