#!/usr/bin/env bash
# Copyright (C) Péter Kardos
# Please refer to the full license distributed with this software.

# Sets up unprivileged access to the storage devices SEDManager talks to
# (SATA/SAS disks via /dev/sd*, NVMe via the /dev/nvme[0-9]* controller
# nodes), without changing device node ownership/group so as not to clobber
# grants installed by other tools:
#   - creates a dedicated group
#   - adds the target user to it
#   - installs a udev rule that grants the group a read/write ACL entry on
#     matching device nodes (additive, does not touch OWNER/GROUP/MODE)
#   - reloads and re-triggers udev so the ACLs apply immediately
# See sed_manager/docs/privilege_separation.md for the rationale.

set -euo pipefail

GROUP_NAME="sedmanager"
RULES_FILE="/etc/udev/rules.d/99-sed-manager.rules"
USER_NAME="${1:-${SUDO_USER:-$USER}}"

if [ "$(id -u)" -ne 0 ]; then
    echo "error: this script must be run as root (e.g. with sudo)" >&2
    exit 1
fi

if ! id "$USER_NAME" >/dev/null 2>&1; then
    echo "error: user '$USER_NAME' does not exist" >&2
    exit 1
fi

SETFACL_PATH="$(command -v setfacl || true)"
if [ -z "$SETFACL_PATH" ]; then
    echo "error: 'setfacl' not found; install the 'acl' package (e.g. 'apt install acl')" >&2
    exit 1
fi

if ! getent group "$GROUP_NAME" >/dev/null; then
    groupadd --system "$GROUP_NAME"
    echo "created group '$GROUP_NAME'"
fi

if ! id -nG "$USER_NAME" | tr ' ' '\n' | grep -qx "$GROUP_NAME"; then
    usermod -aG "$GROUP_NAME" "$USER_NAME"
    echo "added user '$USER_NAME' to group '$GROUP_NAME'"
    echo "note: '$USER_NAME' must log out and back in for the new group membership to take effect"
fi

cat >"$RULES_FILE" <<EOF
# Installed by set_udev_rules.sh - grants the '$GROUP_NAME' group a read/write
# ACL entry on the storage devices SEDManager needs (SATA/SAS disks, NVMe
# controller nodes). Additive: does not change the device node's owner,
# group, or mode, so it composes with other tools' rules for the same nodes.
SUBSYSTEM=="block", KERNEL=="sd*", RUN+="$SETFACL_PATH -m g:$GROUP_NAME:rw- \$env{DEVNAME}"
SUBSYSTEM=="nvme", KERNEL=="nvme[0-9]*", RUN+="$SETFACL_PATH -m g:$GROUP_NAME:rw- \$env{DEVNAME}"
EOF

echo "installed udev rules to '$RULES_FILE'"

udevadm control --reload-rules
udevadm trigger --subsystem-match=block --subsystem-match=nvme

echo "udev rules reloaded and re-triggered"
