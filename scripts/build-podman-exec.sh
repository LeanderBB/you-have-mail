#!/bin/bash

set -eou pipefail

OUTPUT="/opt/artifacts/app-signed.apk"
ALIGNED="/tmp/app-aligned.apk"

source ~/.cargo/env

echo "Cloning repo"
git clone "https://github.com/LeanderBB/you-have-mail.git" /opt/project

cd /opt/project/you-have-mail-android

# Build Project
./gradlew --no-daemon assembleRelease

# Align zip
echo "Aligning apk"
$ANDROID_HOME/build-tools/$ANDROID_BUILD_TOOLS_VERSION/zipalign -v 4 \
    app/build/outputs/apk/release/app-release-unsigned.apk $ALIGNED

# Sign apk
echo "Signing apk"
echo $KEY_STORE_PWD | $ANDROID_HOME/build-tools/$ANDROID_BUILD_TOOLS_VERSION/apksigner sign \
    --ks /opt/keystore \
    --in $ALIGNED \
    --out $OUTPUT

# Verify
echo "Verifying apk"
$ANDROID_HOME/build-tools/$ANDROID_BUILD_TOOLS_VERSION/apksigner verify $OUTPUT

