# Optional Linux artifact builder; dutop runs natively after installation.
FROM rust:1.100.0-slim@sha256:3b199fd6db7d709274df56565b8f4003f9e30d7bfcff9d53148c86053b7855e2 AS build
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools python3 file && rm -rf /var/lib/apt/lists/*
RUN case "$TARGETARCH" in \
      arm64) target=aarch64-unknown-linux-musl ;; \
      amd64) target=x86_64-unknown-linux-musl ;; \
      *) exit 1 ;; \
    esac && rustup target add "$target" && printf '%s' "$target" > /target-triple
RUN useradd --create-home --uid 1000 builder
WORKDIR /work
COPY --chown=builder:builder Cargo.toml Cargo.lock ./
COPY --chown=builder:builder src ./src
COPY --chown=builder:builder tests ./tests
RUN chown builder:builder /work
USER builder
ENV CARGO_HOME=/home/builder/.cargo
RUN target=$(cat /target-triple) && \
    cargo test --locked --target "$target" && \
    cargo build --release --locked --target "$target" && \
    python3 tests/terminal.py "target/$target/release/dutop"
COPY --chown=builder:builder scripts ./scripts
COPY --chown=builder:builder README.md LICENSE ./
COPY --chown=builder:builder packaging ./packaging
RUN target=$(cat /target-triple) && \
    python3 scripts/package.py build --target "$target" --binary "target/$target/release/dutop" && \
    version=$("target/$target/release/dutop" --version | cut -d ' ' -f 2) && \
    python3 tests/package_smoke.py --version "$version" --archive "dist/dutop-$version-$target.tar.gz"

FROM scratch AS artifacts
COPY --from=build /work/dist/ /
