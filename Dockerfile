FROM debian:bookworm

ENV ANDROID_BUILD_TOOLS_VERSION 33.0.1
ENV ANDROID_HOME /opt/android/sdk
ENV ANDROID_NDK_DIR /opt/android/ndk
ENV ANDROID_NDK_HOME $ANDROID_HOME/ndk
ENV ANDROID_VERSION 34
ENV PATH $PATH:$ANDROID_HOME/tools:$ANDROID_HOME/tools/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/bin
ENV ANDROID_NDK_VERSION "25.2.9519653"
ENV RUST_VERSION="1.83.0"


# Enable backports
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list


RUN apt-get update && \
    apt-get install -y --no-install-recommends unzip curl sdkmanager && \
    mkdir -p $ANDROID_HOME $ANDROID_NDK

# Install build deps
RUN apt-get install -y automake clang gcc-multilib libclang-dev \
    libtool make pkg-config python-is-python3 \
    openjdk-17-jdk-headless

# Install go
RUN apt-get install -y -t bookworm-backports golang-go

# Install SDK
RUN yes | sdkmanager --licenses --sdk_root=$ANDROID_HOME && \
    sdkmanager --sdk_root=$ANDROID_HOME \
    "build-tools;${ANDROID_BUILD_TOOLS_VERSION}" \
    "platforms;android-${ANDROID_VERSION}" \
    "platform-tools" \
    "ndk;$ANDROID_NDK_VERSION"

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$RUST_VERSION"

# Cleanup
RUN rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    apt-get autoremove -y && \
    apt-get clean
