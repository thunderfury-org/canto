#!/usr/bin/env bash
# Simulate a LAN gateway with network namespaces and verify canto tproxy.
#
# Requires Linux + root, nft, ip, python3, curl, and a sing-box binary.
#
# From macOS / OrbStack:
#   docker run --rm --privileged -e CARGO_TARGET_DIR=/tmp/canto-target \
#     -v "$PWD":/src -w /src rust:bookworm bash scripts/netns-check.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORKDIR="${WORKDIR:-/tmp/canto-netns-check}"
NS_LAN="canto-check-lan"
NS_GW="canto-check-gw"
NS_WAN="canto-check-wan"
LAN_GW="192.168.100.1"
LAN_HOST="192.168.100.2"
LAN_NET="192.168.100.0/24"
# Must NOT be in reserved_ipv4, otherwise tproxy bypasses it as "LAN dest".
WAN_GW="1.2.3.1"
WAN_HOST="1.2.3.2"
WAN_NET="1.2.3.0/24"
SING_BOX_VERSION="${SING_BOX_VERSION:-1.13.3}"

CANTO_PID=""
HTTP_PID=""

log() { printf '==> %s\n' "$*" >&2; }
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*" >&2; }

need_linux() {
    if [[ "$(uname -s)" != "Linux" ]]; then
        cat >&2 <<EOF
This check must run as root on Linux (nftables + netns).

On this Mac with OrbStack:

  docker run --rm --privileged -e CARGO_TARGET_DIR=/tmp/canto-target \
    -v "$ROOT":/src -w /src rust:bookworm bash scripts/netns-check.sh
EOF
        exit 2
    fi
    if [[ "$(id -u)" -ne 0 ]]; then
        fail "must run as root (or sudo)"
    fi
}

have() { command -v "$1" >/dev/null 2>&1; }

install_deps() {
    local missing=()
    for bin in ip nft python3 curl ss; do
        have "$bin" || missing+=("$bin")
    done
    if [[ ${#missing[@]} -eq 0 ]]; then
        return 0
    fi
    if have apt-get; then
        log "installing: ${missing[*]}"
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -qq
        apt-get install -y -qq nftables iproute2 python3 curl iproute2 procps ca-certificates >/dev/null
    else
        fail "missing tools: ${missing[*]}"
    fi
}

arch_pair() {
    case "$(uname -m)" in
        x86_64 | amd64) echo "amd64" ;;
        aarch64 | arm64) echo "arm64" ;;
        *) fail "unsupported arch: $(uname -m)" ;;
    esac
}

download_sing_box_tar() {
    local tar="$1"
    local file="$WORKDIR/$tar"
    local rel="https://github.com/SagerNet/sing-box/releases/download/v${SING_BOX_VERSION}/${tar}"
    local mirrors=(
        "$rel"
        "https://ghfast.top/${rel}"
        "https://gh-proxy.com/${rel}"
        "https://mirror.ghproxy.com/${rel}"
    )
    local url
    rm -f "$file"
    for url in "${mirrors[@]}"; do
        log "downloading sing-box from ${url}"
        if curl -fL --retry 4 --retry-delay 2 --retry-all-errors --connect-timeout 20             -o "$file" "$url"; then
            [[ -s "$file" ]] && return 0
        fi
    done
    return 1
}

ensure_sing_box() {
    if [[ -n "${SING_BOX:-}" && -x "${SING_BOX}" ]]; then
        echo "$SING_BOX"
        return 0
    fi
    if have sing-box; then
        command -v sing-box
        return 0
    fi
    local arch tar dest
    arch="$(arch_pair)"
    dest="$WORKDIR/sing-box"
    if [[ -x "$dest" ]]; then
        echo "$dest"
        return 0
    fi
    tar="sing-box-${SING_BOX_VERSION}-linux-${arch}.tar.gz"
    if ! download_sing_box_tar "$tar"; then
        log "set SING_BOX to a local sing-box binary if GitHub is unreachable"
        return 1
    fi
    tar -xzf "$WORKDIR/$tar" -C "$WORKDIR"
    cp "$WORKDIR/sing-box-${SING_BOX_VERSION}-linux-${arch}/sing-box" "$dest"
    chmod +x "$dest"
    echo "$dest"
}

ensure_canto() {
    if [[ -n "${CANTO_BIN:-}" && -x "${CANTO_BIN}" ]]; then
        echo "$CANTO_BIN"
        return
    fi
    local target_dir="${CARGO_TARGET_DIR:-$ROOT/target}"
    if [[ -x "$target_dir/debug/canto" ]]; then
        echo "$target_dir/debug/canto"
        return
    fi
    if [[ -x "$target_dir/release/canto" ]]; then
        echo "$target_dir/release/canto"
        return
    fi
    if have cargo; then
        log "building canto"
        (cd "$ROOT" && cargo build --quiet >&2)
        echo "$target_dir/debug/canto"
        return
    fi
    fail "canto binary not found; set CANTO_BIN or run on a rust image"
}

cleanup() {
    if [[ -n "${CANTO_PID:-}" ]] && kill -0 "$CANTO_PID" 2>/dev/null; then
        kill -TERM "$CANTO_PID" 2>/dev/null || true
        sleep 1
        kill -KILL "$CANTO_PID" 2>/dev/null || true
        wait "$CANTO_PID" 2>/dev/null || true
    fi
    if [[ -n "${HTTP_PID:-}" ]] && kill -0 "$HTTP_PID" 2>/dev/null; then
        kill -TERM "$HTTP_PID" 2>/dev/null || true
    fi
    for ns in "$NS_LAN" "$NS_GW" "$NS_WAN"; do
        ip netns del "$ns" 2>/dev/null || true
    done
    ip link del veth-lan 2>/dev/null || true
    ip link del veth-wan 2>/dev/null || true
}
trap cleanup EXIT

setup_netns() {
    ip netns add "$NS_LAN"
    ip netns add "$NS_GW"
    ip netns add "$NS_WAN"

    ip link add veth-lan type veth peer name veth-lan-gw
    ip link set veth-lan netns "$NS_LAN"
    ip link set veth-lan-gw netns "$NS_GW"

    ip link add veth-wan type veth peer name veth-wan-gw
    ip link set veth-wan netns "$NS_WAN"
    ip link set veth-wan-gw netns "$NS_GW"

    ip netns exec "$NS_LAN" bash -c "
        ip link set lo up
        ip addr add ${LAN_HOST}/24 dev veth-lan
        ip link set veth-lan up
        ip route add default via ${LAN_GW}
    "
    ip netns exec "$NS_GW" bash -c "
        ip link set lo up
        ip addr add ${LAN_GW}/24 dev veth-lan-gw
        ip link set veth-lan-gw up
        ip addr add ${WAN_GW}/24 dev veth-wan-gw
        ip link set veth-wan-gw up
        ip route add default via ${WAN_HOST}
    "
    ip netns exec "$NS_WAN" bash -c "
        ip link set lo up
        ip addr add ${WAN_HOST}/24 dev veth-wan
        ip link set veth-wan up
    "
}

write_configs() {
    local sing_box="$1"
    mkdir -p "$WORKDIR/run"
    cat >"$WORKDIR/source.json" <<'JSON'
{
  "log": { "level": "info", "timestamp": true },
  "outbounds": [{ "type": "direct", "tag": "direct" }],
  "route": { "final": "direct" }
}
JSON
    cat >"$WORKDIR/canto.toml" <<EOF
[canto]
work_dir = "$WORKDIR/run"
log_level = "debug"

[singbox]
binary = "$sing_box"
source = "$WORKDIR/source.json"
config_path = "$WORKDIR/run/config.json"
api_listen = "127.0.0.1:9090"

[network]
enabled = true
mode = "tproxy"
tproxy_port = 7893
dns_port = 1053
mixed_port = 7890
fwmark = 424081
routing_mark = 424080
tun_interface = "tun0"
lan_cidrs = ["$LAN_NET"]
EOF
}

wait_for_listen() {
    local i
    for i in $(seq 1 40); do
        if ip netns exec "$NS_GW" bash -c 'ss -lntu | grep -q ":7893"'; then
            return 0
        fi
        sleep 0.25
    done
    return 1
}

need_linux
install_deps
mkdir -p "$WORKDIR"
cleanup || true
trap cleanup EXIT

CANTO_BIN="$(ensure_canto)"
[[ -x "$CANTO_BIN" ]] || fail "canto binary not executable: $CANTO_BIN"
if ! SING_BOX="$(ensure_sing_box)"; then
    fail "could not download sing-box; pass SING_BOX=/path/to/sing-box"
fi
[[ -x "$SING_BOX" ]] || fail "sing-box binary not executable: $SING_BOX"
log "canto=$CANTO_BIN"
log "sing-box=$SING_BOX"

setup_netns
write_configs "$SING_BOX"

ip netns exec "$NS_WAN" python3 -m http.server 80 --bind "$WAN_HOST" >/tmp/canto-wan-http.log 2>&1 &
HTTP_PID=$!
sleep 0.3

log "expect LAN curl to fail before canto (no MASQUERADE)"
if ip netns exec "$NS_LAN" curl -fsS -m 2 "http://${WAN_HOST}/" >/dev/null 2>&1; then
    fail "LAN reached WAN before canto; topology is leaking"
fi
pass "LAN cannot reach WAN before canto"

log "starting canto in gateway netns"
ip netns exec "$NS_GW" "$CANTO_BIN" --config "$WORKDIR/canto.toml" run >"$WORKDIR/canto.log" 2>&1 &
CANTO_PID=$!
if ! wait_for_listen; then
    sed -n '1,80p' "$WORKDIR/canto.log" >&2 || true
    fail "canto/sing-box did not listen on :7893"
fi
pass "canto listening in gw netns"

if ! ip netns exec "$NS_GW" nft list table inet canto >/dev/null; then
    sed -n '1,80p' "$WORKDIR/canto.log" >&2 || true
    fail "nft table inet canto not installed"
fi
pass "nft table inet canto exists"

if ! ip netns exec "$NS_LAN" curl -fsS -m 5 "http://${WAN_HOST}/" >/dev/null; then
    sed -n '1,120p' "$WORKDIR/canto.log" >&2 || true
    fail "LAN tproxy curl http://${WAN_HOST}/ failed"
fi
pass "LAN client reaches WAN via tproxy"

if ! ip netns exec "$NS_GW" curl -fsS -m 5 "http://${WAN_HOST}/" >/dev/null; then
    sed -n '1,120p' "$WORKDIR/canto.log" >&2 || true
    fail "local tproxy curl from gw failed"
fi
pass "gateway process reaches WAN via local tproxy"

if ip netns exec "$NS_WAN" curl -fsS -m 3 "http://${WAN_GW}:7890/" >/dev/null 2>&1; then
    fail "WAN was able to connect to mixed port 7890"
fi
pass "WAN cannot open mixed port"

kill -TERM "$CANTO_PID"
for i in $(seq 1 20); do
    kill -0 "$CANTO_PID" 2>/dev/null || break
    sleep 0.2
done
if kill -0 "$CANTO_PID" 2>/dev/null; then
    kill -KILL "$CANTO_PID" 2>/dev/null || true
fi
wait "$CANTO_PID" 2>/dev/null || true
CANTO_PID=""
sleep 0.2

if ip netns exec "$NS_GW" nft list table inet canto >/dev/null 2>&1; then
    fail "nft table inet canto still present after stop"
fi
pass "nft table removed after SIGTERM"

log "all netns checks passed"
