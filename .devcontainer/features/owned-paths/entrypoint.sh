#!/bin/bash
# Runs as root at every container start, before the container's main command.
set -eu

# shellcheck disable=SC1091
. /usr/local/share/owned-paths/config

for p in $PATHS; do
  [ -e "$p" ] && chown -R "$USERNAME:" "$p"
done

exec "$@"
