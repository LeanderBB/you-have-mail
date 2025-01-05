#/bin/bash
#
set -eou pipefail

mkdir -p artifacts

scripts/podman-do.sh "/opt/scripts/build-podman-exec.sh"

