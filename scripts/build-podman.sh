#/bin/bash
#
set -eou pipefail

mkdir -p artifacts

scripts/podman-do.sh "/opt/project/scripts/build-podman-exec.sh"
