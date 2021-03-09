FROM ubuntu:18.04

ARG DEBIAN_FRONTEND=noninteractive

ARG SOLANA_CHANNEL=v1.2.17
ARG SOLANA_CLI=v1.5.6

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"
ENV PATH="${HOME}/.local/share/solana/install/active_release/bin:${PATH}"

# Install base utilities.
RUN mkdir -p /workdir && mkdir -p /tmp && \
    apt-get update -qq && apt-get upgrade -qq && apt-get install -qq \
    build-essential git curl wget jq pkg-config python3-pip \
    libssl-dev libudev-dev

# Install rust.
RUN curl "https://sh.rustup.rs" -sfo rustup.sh && \
    sh rustup.sh -y && \
    rustup component add rustfmt clippy

# Install Solana tools.
RUN curl -sSf https://raw.githubusercontent.com/solana-labs/solana/${SOLANA_CLI}/install/solana-install-init.sh | sh -s - ${SOLANA_CLI} && \
  # BPF sdk.
  curl -L --retry 5 --retry-delay 2 -o bpf-sdk.tar.bz2 http://solana-sdk.s3.amazonaws.com/${SOLANA_CHANNEL}/bpf-sdk.tar.bz2 && \
  rm -rf bpf-sdk && \
  mkdir -p bpf-sdk && \
  tar jxf bpf-sdk.tar.bz2 && \
  rm -f bpf-sdk.tar.bz2

# Install anchor.
RUN cargo install --git https://github.com/project-serum/anchor anchor-cli --locked

# Build a dummy program to bootstrap the BPF SDK (doing this speeds up builds).
RUN mkdir -p /tmp && cd tmp && anchor init dummy && cd dummy && anchor build

WORKDIR /workdir
