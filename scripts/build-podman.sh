#/bin/bash
#
set -eou pipefail

mkdir -p artifacts

podman run --rm --volume $PWD:/opt/project:Z \
    --volume $PWD/artifacts:/opt/artifacts:rw \
    --secret android_keystore,type=mount,target=/opt/keystore \
    --secret android_key_pwd,type=env,target=KEY_STORE_PWD \
    -i -t yhm "/opt/project/scripts/build-podman-exec.sh"

