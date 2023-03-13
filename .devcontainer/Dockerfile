FROM mcr.microsoft.com/devcontainers/rust:0-1-bullseye

# install docker to run docker commands inside the container
RUN apt update && \
    apt install -qy curl && \
    curl -sSL https://get.docker.com/ | sh
