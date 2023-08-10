FROM centos:7

RUN ulimit -n 1024 && yum install -y gcc

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
  && chmod +x rustup.sh \
  && ./rustup.sh -y \
  && rm rustup.sh

RUN mkdir /app

WORKDIR /app

COPY app/Cargo.* app/dummy.rs /app
RUN sed s#src/main.rs#dummy.rs#g -i Cargo.toml

ENV PATH="$PATH:~/.cargo/bin/"

RUN cargo build --release
