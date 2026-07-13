# Build stage. protoc compiles hiero-streams' vendored HAPI protos; those
# import protobuf's well-known types (google/protobuf/wrappers.proto, ...),
# which ship in libprotobuf-dev, not the protobuf-compiler package — both are
# required or the build script fails with "wrappers.proto: File not found".
# The crate comes from crates.io, so this build is self-contained.
FROM rust:1.97-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
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
