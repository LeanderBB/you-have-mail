#/bin/bash
#
set -eou pipefail

HOST_UID=$(id -u)

podman run --rm \
  --userns=keep-id \
  --volume $PWD:/opt/project:Z \
  --mount type=tmpfs,destination=/opt/project/target \
  --volume $PWD/artifacts:/opt/artifacts:Z \
  --secret android_keystore,type=mount,target=/opt/keystore \
  --secret android_key_pwd,type=env,target=KEY_STORE_PWD \
  -i -t yhm:uid-${HOST_UID} $@
