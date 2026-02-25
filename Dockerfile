FROM debian:bookworm

ARG USER_ID=1000
ARG GROUP_ID=1000

ENV ANDROID_BUILD_TOOLS_VERSION 35.0.0
ENV ANDROID_HOME /opt/android/sdk
ENV ANDROID_NDK_DIR $ANDROID_HOME/ndk
ENV ANDROID_NDK_HOME $ANDROID_HOME/ndk
ENV ANDROID_VERSION 36
ENV PATH $PATH:$ANDROID_HOME/tools:$ANDROID_HOME/tools/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/bin
ENV ANDROID_NDK_VERSION "28.2.13676358"
ENV RUST_VERSION="1.88.0"
ENV CARGO_HOME /opt/cargo
ENV RUSTUP_HOME /opt/rustup

RUN groupadd -g ${GROUP_ID} builder && \
  useradd -l -m -u ${USER_ID} -g ${GROUP_ID} builder


RUN apt-get update && \
  apt-get install -y --no-install-recommends unzip curl sdkmanager

# Install build deps
RUN apt-get install -y automake clang gcc-multilib libclang-dev \
  libtool make pkg-config python-is-python3 \
  openjdk-17-jdk-headless git

# Create other dirs
RUN mkdir -p ${ANDROID_HOME} ${CARGO_HOME} ${RUSTUP_HOME} && chown -R builder:builder ${ANDROID_HOME} ${CARGO_HOME} ${RUSTUP_HOME}


# Cleanup
RUN rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
  apt-get autoremove -y && \
  apt-get clean

USER builder

# Install SDK
RUN yes | sdkmanager --licenses --sdk_root=$ANDROID_HOME && \
  sdkmanager --sdk_root=$ANDROID_HOME \
  "build-tools;${ANDROID_BUILD_TOOLS_VERSION}" \
  "platforms;android-${ANDROID_VERSION}" \
  "platform-tools" \
  "ndk;$ANDROID_NDK_VERSION"

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$RUST_VERSION"

RUN ${CARGO_HOME}/bin/rustup component add rust-std-aarch64-linux-android rust-std-armv7-linux-androideabi rust-std-x86_64-linux-android

