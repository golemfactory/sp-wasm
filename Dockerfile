FROM rust:1.38

RUN echo "deb http://deb.debian.org/debian stretch-backports main" >> /etc/apt/sources.list
RUN apt -y update && apt -y install autoconf2.13 clang-6.0 --no-install-recommends && rm -rf /var/lib/apt/lists/*

WORKDIR /sp-wasm
COPY . .
ENV SHELL=/bin/bash
ENV CC=clang-6.0
ENV CPP="clang-6.0 -E"
ENV CXX=clang++-6.0
RUN cargo install --path .
RUN cargo clean

ENTRYPOINT ["wasm-sandbox"]
