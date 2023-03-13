FROM rust:bullseye

# install docker inside the image in order to send shell commands to wireguard container
RUN apt update && \
    apt install -qy curl && \
    curl -sSL https://get.docker.com/ | sh

ENTRYPOINT ["tail", "-f", "/dev/null"]
