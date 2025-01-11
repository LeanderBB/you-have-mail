#/bin/bash
#
set -eou pipefail

podman run --rm \
    --volume $PWD/scripts:/opt/scripts:O \
    --volume $PWD/artifacts:/opt/artifacts:Z \
    --secret android_keystore,type=mount,target=/opt/keystore \
    --secret android_key_pwd,type=env,target=KEY_STORE_PWD \
    -i -t yhm $@

