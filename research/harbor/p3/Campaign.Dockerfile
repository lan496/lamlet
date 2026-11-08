ARG RUNTIME_IMAGE
FROM rust:1.85-bookworm@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4 AS compiler
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked --jobs 2

FROM ${RUNTIME_IMAGE} AS common
USER root
RUN apt-get update && apt-get install -y --no-install-recommends curl git xz-utils ripgrep && rm -rf /var/lib/apt/lists/*
USER agent

FROM common AS checker
USER root
COPY --from=compiler /source/target/release/lamlet /usr/local/bin/lamlet
USER agent
