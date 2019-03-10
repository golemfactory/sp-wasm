FROM rust:1.33

RUN echo "deb http://deb.debian.org/debian stretch-backports main" >> /etc/apt/sources.list
RUN apt -y update
RUN apt -y install autoconf2.13 clang-6.0

WORKDIR /sp-wasm
COPY . .
ENV SHELL=/bin/bash
ENV CC=clang-6.0
ENV CPP="clang-6.0 -E"
ENV CXX=clang++-6.0
RUN cargo install --path .

ENTRYPOINT [ "sp_wasm" ]
