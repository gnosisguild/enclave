ARG RISC0_TOOLCHAIN=1.91.1

# Extract the checksum-pinned host compiler without changing the Rust base image's packages.
FROM busybox:1.37.0-musl@sha256:dc88b80842580654294472735332ec7c7789a4010ec9a68b69f9b88c19abede6 AS protoc
ADD --checksum=sha256:af27ea66cd26938fe48587804ca7d4817457a08350021a1c6e23a27ccc8c6904 \
    https://github.com/protocolbuffers/protobuf/releases/download/v34.1/protoc-34.1-linux-x86_64.zip /tmp/protoc.zip
RUN unzip -q /tmp/protoc.zip -d /protoc

# Keep the base image content-addressed. The tag documents the matching RISC Zero Rust
# toolchain; the digest prevents a registry-side tag change from changing the guest ELF.
FROM risczero/risc0-guest-builder:r0.${RISC0_TOOLCHAIN}@sha256:fafb377a44e1cfca415577c48d2f7012bda99ed36f2fae27f9a663b9fe6048f0
COPY --from=protoc /protoc/bin/protoc /usr/local/bin/protoc
COPY --from=protoc /protoc/include /usr/local/include
RUN ["/usr/local/bin/protoc", "--version"]
