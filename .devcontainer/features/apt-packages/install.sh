#!/bin/bash
set -eu

PACKAGES="${PACKAGES:-}"
[ -n "$PACKAGES" ] || exit 0

export DEBIAN_FRONTEND=noninteractive
apt-get update
# shellcheck disable=SC2086
apt-get install -y --no-install-recommends $PACKAGES
apt-get clean
rm -rf /var/lib/apt/lists/*
