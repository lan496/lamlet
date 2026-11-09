ARG BASELINE_IMAGE
FROM rust:1.85-bookworm@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4 AS compiler
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked --jobs 2

FROM ${BASELINE_IMAGE} AS common
USER root
COPY repair_instrument.py /opt/repair_instrument.py
RUN chmod 0555 /opt/repair_instrument.py && ln -s /opt/repair_instrument.py /usr/local/bin/lmp
USER agent

FROM common AS checker
USER root
COPY --from=compiler /source/target/release/lamlet /opt/lamlet/lamlet
RUN ln -s /opt/repair_instrument.py /usr/local/bin/lamlet
USER agent
