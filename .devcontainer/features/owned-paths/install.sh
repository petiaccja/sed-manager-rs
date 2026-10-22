#!/bin/bash
set -eu

DIR=/usr/local/share/owned-paths
mkdir -p "$DIR"
cp entrypoint.sh "$DIR/"
chmod 755 "$DIR/entrypoint.sh"
cat > "$DIR/config" <<CONF
USERNAME="${USERNAME:-vscode}"
PATHS="${PATHS:-}"
CONF

# Workaround for a podman/buildah layer-commit quirk: when an install step does not
# modify /tmp, the committed layer records /tmp as mode 755. That breaks apt in later
# features and non-root writes to /tmp at runtime. Creating and removing a file in
# /tmp makes the layer record the correct mode (1777).
touch /tmp/.layer-fix && rm -f /tmp/.layer-fix
