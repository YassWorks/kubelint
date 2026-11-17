#!/usr/bin/env bash
#
# --- E2E ---
#
# Starts a real Kubernetes API server (etcd and kube-apiserver, no kubelet) for end to
# end checks, and the Docker daemon for running containers.
# Usage: .github/scripts/e2e-apiserver.sh [VERSION]
# VERSION is a minor (1.34), a release (v1.34.13 or 1.34.13), or empty for the binaries
# on the PATH, else the latest release. Missing binaries are downloaded into
# target/e2e/bin. Prints the export line for KUBECONFIG and PATH. One server runs per
# checkout, on ports free when it starts: a call for the same version reuses it, a call
# for another replaces it, and other checkouts keep their own.
#
# -----------
#
set -euo pipefail

etcd_version=v3.6.16
root="$(cd "$(dirname "$0")/../.." && pwd)/target/e2e"
state="$root/state"
case "$(uname -m)" in
  x86_64) arch=amd64 ;;
  aarch64 | arm64) arch=arm64 ;;
  *) echo "unsupported architecture $(uname -m)" >&2; exit 1 ;;
esac

fetch() {
  local url=$1 target=$2
  if ! curl -fsSLo "$target.part" "$url"; then
    rm -f "$target.part"
    return 1
  fi
  chmod +x "$target.part"
  mv "$target.part" "$target"
}

version="${1:-}"
if [ -z "$version" ] && command -v kube-apiserver >/dev/null \
  && command -v kubectl >/dev/null; then
  bin="$(dirname "$(command -v kube-apiserver)")"
  kubectl_bin="$(command -v kubectl)"
else
  case "$version" in
    "") version="$(curl -fsSL https://dl.k8s.io/release/stable.txt)" ;;
    v*.*.*) ;;
    [0-9]*.*.*) version="v$version" ;;
    *)
      version="$(curl -fsSL "https://dl.k8s.io/release/stable-${version#v}.txt")" \
        || { echo "unknown Kubernetes version $1" >&2; exit 1; }
      ;;
  esac
  bin="$root/bin/$version"
  kubectl_bin="$bin/kubectl"
  mkdir -p "$bin"
  for tool in kube-apiserver kubectl; do
    if [ ! -x "$bin/$tool" ]; then
      fetch "https://dl.k8s.io/release/$version/bin/linux/$arch/$tool" "$bin/$tool" \
        || { rmdir "$bin" 2>/dev/null; echo "unknown Kubernetes version $1" >&2; exit 1; }
    fi
  done
fi

if command -v etcd >/dev/null; then
  etcd_bin="$(command -v etcd)"
else
  etcd_bin="$root/bin/etcd-$etcd_version/etcd"
  if [ ! -x "$etcd_bin" ]; then
    mkdir -p "$(dirname "$etcd_bin")"
    name="etcd-$etcd_version-linux-$arch"
    curl -fsSL "https://github.com/etcd-io/etcd/releases/download/$etcd_version/$name.tar.gz" \
      | tar xz -C "$(dirname "$etcd_bin")" --strip-components=1 "$name/etcd"
  fi
fi

if ! docker info >/dev/null 2>&1; then
  if command -v dockerd >/dev/null; then
    mkdir -p "$root"
    dockerd >"$root/dockerd.log" 2>&1 &
    for _ in $(seq 1 30); do docker info >/dev/null 2>&1 && break; sleep 1; done
    docker info >/dev/null 2>&1 || echo "warning: Docker did not start, see $root/dockerd.log" >&2
  else
    echo "warning: dockerd not found, containers cannot run" >&2
  fi
fi

server_version() {
  KUBECONFIG="$state/kubeconfig" "$kubectl_bin" get --raw /version 2>/dev/null \
    | sed -n 's/.*"gitVersion": "\([^"]*\)".*/\1/p' || true
}
print_exports() {
  printf 'export KUBECONFIG=%q PATH=%q:"%s"\n' \
    "$state/kubeconfig" "$(dirname "$kubectl_bin")" "\$PATH"
}

wanted="$("$bin/kube-apiserver" --version | cut -d' ' -f2)"
if [ "$(server_version)" = "$wanted" ]; then
  echo "API server $wanted already running" >&2
  print_exports
  exit 0
fi

ours="--data-dir $state/etcd|--cert-dir $state/certs"
port_open() { (echo >"/dev/tcp/127.0.0.1/$1") 2>/dev/null; }
free_port() {
  local port=$1
  while port_open "$port"; do port=$((port + 1)); done
  echo "$port"
}
fail() {
  echo "$1" >&2
  pkill -KILL -f -- "$ours" || true
  exit 1
}
# One start at a time on the machine, so two checkouts never pick the same free ports.
exec 9>"${TMPDIR:-/tmp}/kubelint-e2e.lock"
flock 9
pkill -KILL -f -- "$ours" || true
rm -rf "$state"
mkdir -p "$state"
etcd_port="$(free_port 2379)"
peer_port="$(free_port $((etcd_port + 1)))"
api_port="$(free_port 6443)"

openssl genrsa -out "$state/sa.key" 2048 2>/dev/null
token="$(openssl rand -hex 16)"
echo "$token,admin,admin,system:masters" >"$state/tokens.csv"
"$etcd_bin" --data-dir "$state/etcd" --listen-client-urls "http://127.0.0.1:$etcd_port" \
  --advertise-client-urls "http://127.0.0.1:$etcd_port" \
  --listen-peer-urls "http://127.0.0.1:$peer_port" >"$state/etcd.log" 2>&1 9>&- &
echo $! >"$state/etcd.pid"
# Real clusters allow privileged pods, and no controller creates service accounts here.
"$bin/kube-apiserver" --etcd-servers "http://127.0.0.1:$etcd_port" --cert-dir "$state/certs" \
  --secure-port "$api_port" --bind-address 127.0.0.1 --token-auth-file "$state/tokens.csv" \
  --authorization-mode RBAC --service-account-issuer https://kubernetes.default.svc \
  --service-account-key-file "$state/sa.key" \
  --service-account-signing-key-file "$state/sa.key" \
  --service-cluster-ip-range 10.96.0.0/16 --allow-privileged=true \
  --disable-admission-plugins ServiceAccount >"$state/apiserver.log" 2>&1 9>&- &
echo $! >"$state/apiserver.pid"

cat >"$state/kubeconfig" <<EOF
apiVersion: v1
kind: Config
clusters:
- name: e2e
  cluster: {server: "https://127.0.0.1:$api_port", insecure-skip-tls-verify: true}
users: [{name: admin, user: {token: $token}}]
contexts: [{name: e2e, context: {cluster: e2e, user: admin}}]
current-context: e2e
EOF

for _ in $(seq 1 60); do
  for name in etcd apiserver; do
    kill -0 "$(cat "$state/$name.pid")" 2>/dev/null || fail "$name exited, see $state/$name.log"
  done
  if [ "$(server_version)" = "$wanted" ] \
    && KUBECONFIG="$state/kubeconfig" "$kubectl_bin" get namespace default >/dev/null 2>&1; then
    echo "API server $wanted ready" >&2
    print_exports
    exit 0
  fi
  sleep 1
done
fail "API server did not become ready, see $state/apiserver.log"
