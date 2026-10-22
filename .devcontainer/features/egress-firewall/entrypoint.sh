#!/bin/bash
# Runs as root at every container start, before the container's main command.
# Sets up the egress firewall and fails closed: if anything goes wrong the
# container does not start.
set -eu

DIR=/usr/local/share/egress-firewall
STATE=/var/lib/egress-firewall
RUN=/run/egress-firewall
DNS_USER=egress-dns

log() { echo "[egress-firewall] $*" >&2; }

# --- Upstream resolvers (taken from what the container runtime configured) ----
mkdir -p "$STATE" "$RUN"
upstream=$(awk '$1=="nameserver" && $2!="127.0.0.1" && $2!="::1" {print $2}' /etc/resolv.conf)
if [ -n "$upstream" ]; then
  echo "$upstream" > "$STATE/upstream"
else
  upstream=$(cat "$STATE/upstream" 2>/dev/null || true)
fi
[ -n "$upstream" ] || { log "no upstream resolver found"; exit 1; }

# --- Extra allowed endpoints (URLs in $EGRESS_ALLOW_URLS), resolved now -------
# Needed for e.g. a telemetry collector on the host, which is not a DNS name.
extra_rules=""
for url in ${EGRESS_ALLOW_URLS:-}; do
  scheme=${url%%://*}
  hostport=${url#*://}; hostport=${hostport%%/*}
  host=${hostport%%:*}
  port=${hostport##*:}
  [ "$port" = "$hostport" ] && { [ "$scheme" = https ] && port=443 || port=80; }
  for ip in $(getent ahostsv4 "$host" | awk '{print $1}' | sort -u); do
    extra_rules="$extra_rules
    ip daddr $ip tcp dport $port accept"
  done
  log "allowing $host:$port"
done

# --- nftables -----------------------------------------------------------------
up4=""; up6=""
for ns in $upstream; do
  case "$ns" in *:*) up6="$up6${up6:+, }$ns" ;; *) up4="$up4${up4:+, }$ns" ;; esac
done
dns_rules=""
[ -n "$up4" ] && dns_rules="$dns_rules
    meta skuid \"$DNS_USER\" ip daddr { $up4 } meta l4proto { udp, tcp } th dport 53 accept"
[ -n "$up6" ] && dns_rules="$dns_rules
    meta skuid \"$DNS_USER\" ip6 daddr { $up6 } meta l4proto { udp, tcp } th dport 53 accept"

nft add table inet egress
nft delete table inet egress
nft -f - <<NFT
table inet egress {
  set allowed4 { type ipv4_addr; }
  set allowed6 { type ipv6_addr; }
  chain output {
    type filter hook output priority 0; policy drop;
    oif "lo" accept
    ct state established,related accept$dns_rules
    ip daddr @allowed4 accept
    ip6 daddr @allowed6 accept$extra_rules
    meta l4proto tcp reject with tcp reset
    reject
  }
}
NFT

# --- dnsmasq: resolves only allowed domains and feeds their IPs to nftables ---
conf="$RUN/dnsmasq.conf"
{
  echo "no-resolv"
  echo "listen-address=127.0.0.1"
  echo "bind-interfaces"
  echo "user=$DNS_USER"
  echo "group=$DNS_USER"
  echo "cache-size=1000"
  echo "log-queries"
  echo "log-facility=/var/log/egress-dnsmasq.log"
  echo "pid-file=$RUN/dnsmasq.pid"
  grep -v '^\s*\(#\|$\)' "$DIR/allowlist.txt" | while read -r domain; do
    for ns in $upstream; do echo "server=/$domain/$ns"; done
    echo "nftset=/$domain/4#inet#egress#allowed4,6#inet#egress#allowed6"
  done
} > "$conf"
: > /var/log/egress-dnsmasq.log
chmod 644 /var/log/egress-dnsmasq.log
dnsmasq --conf-file="$conf"
kill -0 "$(cat "$RUN/dnsmasq.pid")"

printf 'nameserver 127.0.0.1\n' > /etc/resolv.conf
log "active: $(grep -c '^nftset=' "$conf") allowed domains, upstream resolvers: $(echo $upstream)"

exec "$@"
