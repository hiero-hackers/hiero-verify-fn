# Build stage. protoc is for hiero-streams' generated protobuf modules.
# The crate comes from crates.io, so this build is self-contained.
FROM rust:1.88-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release

# Runtime: distroless, nothing but the 2 MB binary and the genesis block.
FROM gcr.io/distroless/cc-debian12
COPY --from=build /src/target/release/hiero-verify-fn /hiero-verify-fn
COPY bootstrap/genesis-cn-0.73-tss.blk.gz /genesis.blk.gz
ENV BOOTSTRAP_BLOCK=/genesis.blk.gz
ENTRYPOINT ["/hiero-verify-fn"]
