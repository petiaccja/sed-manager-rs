#!/bin/bash
# Build-time (root) installation of the egress firewall.
set -eu

DIR=/usr/local/share/egress-firewall

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends dnsmasq-base nftables ca-certificates curl

# Dedicated user for dnsmasq: the firewall only lets this uid talk to the upstream resolver.
id egress-dns >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin egress-dns

mkdir -p "$DIR"
cp entrypoint.sh allowlist.txt "$DIR/"
chmod 755 "$DIR/entrypoint.sh"
chmod 644 "$DIR/allowlist.txt"

# Without root, the unprivileged user must not be able to change the firewall.
rm -f /etc/sudoers.d/*
apt-get purge -y sudo || true

apt-get clean
rm -rf /var/lib/apt/lists/*
