#!/usr/bin/env bash
# Copyright (C) Péter Kardos
# Please refer to the full license distributed with this software.

# Grants the executable the Linux capabilities it needs to issue TCG security
# commands (ATA/SCSI/NVMe passthrough ioctls) without running as root.
# See sed_manager/docs/privilege_separation.md for the rationale behind the
# chosen set of capabilities.

set -euo pipefail

TARGET="${1:-./sed_manager_gui}"

if [ ! -f "$TARGET" ]; then
    echo "error: '$TARGET' does not exist or is not a file" >&2
    exit 1
fi

setcap 'cap_sys_rawio,cap_sys_admin+eip' "$TARGET"
