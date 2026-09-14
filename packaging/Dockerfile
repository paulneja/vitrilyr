FROM ubuntu:24.04 AS builder
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl git pkg-config libgtk-4-dev \
    libwayland-dev wayland-protocols meson ninja-build dbus xvfb \
    fonts-noto-cjk file binutils && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --default-toolchain 1.95.0
ENV PATH="/root/.cargo/bin:${PATH}"
RUN git clone https://github.com/wmww/gtk4-layer-shell.git /tmp/layer-shell && \
    git -C /tmp/layer-shell checkout --detach 1c963c51514581c41b9bdae08cdf69171265cdda && \
    meson setup /tmp/layer-shell/build /tmp/layer-shell --prefix=/opt/layer-shell \
        --libdir=lib --default-library=static -Dintrospection=false -Dvapi=false \
        -Dexamples=false -Dtests=false -Ddocs=false && \
    meson compile -C /tmp/layer-shell/build && meson install -C /tmp/layer-shell/build
ENV PKG_CONFIG_PATH=/opt/layer-shell/lib/pkgconfig
ENV SYSTEM_DEPS_GTK4_LAYER_SHELL_0_LINK=static
# GTK must resolve the Wayland hooks from the executable, even with a static archive.
ENV RUSTFLAGS="-C link-arg=-Wl,--export-dynamic-symbol=wl_proxy_*"
ENV CARGO_BUILD_JOBS=2
WORKDIR /src
COPY . .
RUN cargo test --locked && ./scripts/build.sh --wayland && \
    VITRILYR_TEST_BINARY=/src/target/release/vitrilyr cargo test --locked --test x11 -- --ignored
RUN mkdir -p /out && install -m755 target/release/vitrilyr /out/vitrilyr-linux-x86_64 && \
    file /out/vitrilyr-linux-x86_64 && \
    ! readelf -d /out/vitrilyr-linux-x86_64 | grep -q 'libgtk4-layer-shell.so' && \
    ldd /out/vitrilyr-linux-x86_64 > /out/runtime-libraries.txt && \
    ! grep -q 'not found' /out/runtime-libraries.txt && \
    cp LICENSE /out/LICENSE && cp /tmp/layer-shell/LICENSE /out/LICENSE-gtk4-layer-shell && \
    cp packaging/RELEASE.md /out/README.md && \
    cd /out && sha256sum vitrilyr-linux-x86_64 > SHA256SUMS

FROM scratch AS artifact
COPY --from=builder /out/ /
