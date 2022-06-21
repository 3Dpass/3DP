FROM rust:1.60 as builder
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libclang-dev
RUN rustup update nightly && \
    rustup target add wasm32-unknown-unknown --toolchain nightly
ADD . ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target/release/build \
    --mount=type=cache,target=/app/target/release/deps \
    --mount=type=cache,target=/app/target/release/.fingerprint \
    --mount=type=cache,target=/app/target/release/wbuild \
    cargo build --bin poscan-consensus --release

FROM ubuntu:kinetic
ARG APP=/usr/src/app
COPY --from=builder /app/target/release/poscan-consensus ${APP}/p3d
COPY ./testnetSpecRaw.json ${APP}/testnetSpecRaw.json
COPY ./docker/node.sh ${APP}/node.sh
RUN chmod +x ${APP}/node.sh
WORKDIR ${APP}
EXPOSE 9933
EXPOSE 9944

CMD ["./node.sh"]
