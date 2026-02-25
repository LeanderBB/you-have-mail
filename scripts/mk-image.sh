#!/bin/sh

set -eou pipefail

HOST_UID=$(id -u)
HOST_GID=$(id -g)

podman build \
  --build-arg USER_ID=${HOST_UID} \
  --build-arg GROUP_ID=${HOST_GID} \
  -t yhm:uid-${HOST_UID} .
