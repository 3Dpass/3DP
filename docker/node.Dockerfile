FROM rust:1.71 AS builder
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libclang-dev cmake
RUN rustup update nightly && \
    rustup default nightly-2023-05-05 && \
    rustup target add wasm32-unknown-unknown --toolchain nightly-2023-05-05
ADD . ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target/release/build \
    --mount=type=cache,target=/app/target/release/deps \
    --mount=type=cache,target=/app/target/release/.fingerprint \
    --mount=type=cache,target=/app/target/release/wbuild \
    cargo build --release

FROM ubuntu:kinetic
ARG APP=/usr/src/app
COPY --from=builder /app/target/release/poscan-consensus ${APP}/p3d
COPY ./mainnetSpecRaw.json ${APP}/mainnetSpecRaw.json
COPY ./docker/node.sh ${APP}/node.sh
RUN chmod +x ${APP}/node.sh
WORKDIR ${APP}
EXPOSE 9933
EXPOSE 9944
EXPOSE 30333

CMD ["./node.sh"]
