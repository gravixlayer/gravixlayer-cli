#!/usr/bin/env bash
# Live E2E for storage mounts driven through the CLI — buckets, bucket links,
# git mounts, lifecycle, credential hygiene. Requires a built `gravixlayer`
# binary, jq, and a live deployment.
#
# Config — env or ~/.config/gravix-test/aws.env:
#   GRAVIXLAYER_API_KEY  (required)   GRAVIXLAYER_BASE_URL  (required)
#   GRAVIXLAYER_TEMPLATE (default base-small)
#   TEST_S3_BUCKET  TEST_S3_REGION  AWS_ACCESS_KEY_ID  AWS_SECRET_ACCESS_KEY
#   PUBLIC_REPO  PRIVATE_REPO  GIT_TOKEN
#
#   ./scripts/test_storage_e2e.sh
set -u -o pipefail

ENV_FILE="${HOME}/.config/gravix-test/aws.env"
if [[ -f "$ENV_FILE" ]]; then
  while IFS='=' read -r k v; do
    [[ "$k" =~ ^[A-Z_]+$ ]] || continue
    v="${v%\"}"; v="${v#\"}"; v="${v%\'}"; v="${v#\'}"
    [[ -z "${!k:-}" ]] && export "$k=$v"
  done < "$ENV_FILE"
fi

GX="${GRAVIXLAYER_BIN:-gravixlayer}"
BASE_URL="${GRAVIXLAYER_BASE_URL:-}"
TEMPLATE="${GRAVIXLAYER_TEMPLATE:-base-small}"
TEST_BUCKET="${TEST_S3_BUCKET:-demo-gl-157541785332-ap-south-1-an}"
TEST_REGION="${TEST_S3_REGION:-ap-south-1}"
RUN_ID="$(head -c 8 /dev/urandom | od -An -tx1 | tr -d ' \n' | head -c 8)"

if [[ -z "${GRAVIXLAYER_API_KEY:-}" || -z "$BASE_URL" ]]; then
  echo "SKIP: GRAVIXLAYER_API_KEY and GRAVIXLAYER_BASE_URL required" >&2
  exit 0
fi
command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }

PASS=0; FAIL=0
RUNTIMES=(); BUCKETS=(); LINKS=(); PROVIDERS=()

gxj() { "$GX" --base-url "$BASE_URL" --output json "$@"; }
ok()   { PASS=$((PASS+1)); echo "  PASS $1"; }
bad()  { FAIL=$((FAIL+1)); echo "  FAIL $1"; }
chk()  { if "$@"; then :; else bad "$*"; fi; }

# guest exec — prints stdout; non-zero guest exit propagates
exec_in() { gxj runtime exec "$1" -- sh -c "$2" 2>/dev/null; }

wait_status() { # id want [timeout]
  local id="$1" want="$2" t="${3:-240}" now
  local end=$((SECONDS + t))
  while (( SECONDS < end )); do
    now="$(gxj runtime get "$id" | jq -r '.status // empty')"
    [[ "$now" == "$want" ]] && return 0
    [[ "$now" =~ ^(error|killed|stopped|dead)$ ]] && return 1
    sleep 3
  done
  return 1
}

wait_mount() { # rid path want [timeout]
  local rid="$1" path="$2" want="${3:-attached}" t="${4:-180}" st
  local end=$((SECONDS + t))
  while (( SECONDS < end )); do
    st="$(gxj runtime mounts list "$rid" |
      jq -r --arg p "$path" '.mounts[]? | select(.mount_path==$p) | .state')"
    [[ "$st" == "$want" ]] && return 0
    sleep 3
  done
  return 1
}

cleanup() {
  echo "== cleanup =="
  for r in "${RUNTIMES[@]:-}"; do gxj runtime kill "$r" >/dev/null 2>&1; done
  sleep 2
  for l in "${LINKS[@]:-}";     do gxj storage link delete "$l" >/dev/null 2>&1; done
  for b in "${BUCKETS[@]:-}";   do gxj storage bucket delete "$b" >/dev/null 2>&1; done
  for p in "${PROVIDERS[@]:-}"; do gxj provider delete "$p" >/dev/null 2>&1; done
}
trap cleanup EXIT

# ---------------------------------------------------------------- bucket ----

echo "== managed bucket =="
BUCKET_JSON="$(gxj storage bucket create "e2e-$RUN_ID" --region "$TEST_REGION")"
BUCKET_ID="$(jq -r '.id' <<<"$BUCKET_JSON")"
[[ -n "$BUCKET_ID" && "$BUCKET_ID" != null ]] && ok "bucket create" || bad "bucket create: $BUCKET_JSON"
BUCKETS+=("$BUCKET_ID")

gxj storage bucket list | jq -e --arg id "$BUCKET_ID" '.buckets[] | select(.id==$id)' \
  >/dev/null && ok "bucket list" || bad "bucket list"
[[ "$(gxj storage bucket get "$BUCKET_ID" | jq -r '.id')" == "$BUCKET_ID" ]] \
  && ok "bucket get" || bad "bucket get"

# wait for active
end=$((SECONDS + 240)); BSTATUS=""
while (( SECONDS < end )); do
  BSTATUS="$(gxj storage bucket get "$BUCKET_ID" | jq -r '.status')"
  [[ "$BSTATUS" =~ ^(active|failed)$ ]] && break
  sleep 3
done
[[ "$BSTATUS" == "active" ]] && ok "bucket active" || bad "bucket never active: $BSTATUS"

# create-time rw mount
RT1="$(gxj runtime create --template "$TEMPLATE" --wait \
  --mount "{\"kind\":\"bucket\",\"path\":\"/mnt/bucket\",\"bucket\":\"$BUCKET_ID\",\"mode\":\"rw\"}" \
  | jq -r '.runtime_id // .id')"
[[ -n "$RT1" && "$RT1" != null ]] && ok "runtime create+mount" || bad "runtime create"
RUNTIMES+=("$RT1")
wait_status "$RT1" running && ok "runtime running" || bad "runtime not running"
wait_mount "$RT1" /mnt/bucket && ok "bucket attached" || bad "bucket not attached"

exec_in "$RT1" "echo hi-$RUN_ID > /mnt/bucket/f.txt && sync && cat /mnt/bucket/f.txt" \
  | grep -q "hi-$RUN_ID" && ok "guest write+read" || bad "guest write+read"
exec_in "$RT1" "mkdir -p /mnt/bucket/d && mv /mnt/bucket/f.txt /mnt/bucket/d/m.txt && rm /mnt/bucket/d/m.txt && rmdir /mnt/bucket/d" \
  && ok "guest rename/delete" || bad "guest rename/delete"
if [[ -n "${AWS_ACCESS_KEY_ID:-}" ]]; then
  ! exec_in "$RT1" "env; cat /proc/*/environ 2>/dev/null | tr '\0' '\n'" \
    | grep -q "$AWS_ACCESS_KEY_ID" && ok "no aws creds in guest" \
    || bad "AWS ACCESS KEY leaked into guest env"
fi

# ro mount rejects writes — second runtime proves the object is really in S3
exec_in "$RT1" "echo p2-$RUN_ID > /mnt/bucket/p2.txt && sync" || bad "seed p2"
RT2="$(gxj runtime create --template "$TEMPLATE" --wait \
  --mount "{\"kind\":\"bucket\",\"path\":\"/mnt/ro\",\"bucket\":\"$BUCKET_ID\",\"mode\":\"ro\"}" \
  | jq -r '.runtime_id // .id')"
RUNTIMES+=("$RT2")
wait_status "$RT2" running && wait_mount "$RT2" /mnt/ro \
  && ok "ro mount attached" || bad "ro mount"
exec_in "$RT2" "cat /mnt/ro/p2.txt" | grep -q "p2-$RUN_ID" \
  && ok "cross-runtime S3 read" || bad "cross-runtime read"
! exec_in "$RT2" "touch /mnt/ro/nope.txt" >/dev/null 2>&1 \
  && ok "ro rejects write" || bad "ro write succeeded"

# delete refused while mounted
gxj storage bucket delete "$BUCKET_ID" >/dev/null 2>&1 \
  && bad "bucket delete allowed while mounted" || ok "delete refused while mounted"

# hot attach + detach
RT3="$(gxj runtime create --template "$TEMPLATE" --wait | jq -r '.runtime_id // .id')"
RUNTIMES+=("$RT3")
wait_status "$RT3" running || bad "rt3 not running"
ATT="$(gxj runtime mounts attach "$RT3" \
  --mount "{\"kind\":\"bucket\",\"path\":\"/mnt/hot\",\"bucket\":\"$BUCKET_ID\"}" \
  | jq -r '.id')"
[[ -n "$ATT" && "$ATT" != null ]] && ok "hot attach accepted" || bad "hot attach"
wait_mount "$RT3" /mnt/hot && ok "hot mount live" || bad "hot mount"
exec_in "$RT3" "echo h > /mnt/hot/h.txt && cat /mnt/hot/h.txt" | grep -q h \
  && ok "hot mount io" || bad "hot mount io"
gxj runtime mounts detach "$RT3" "$ATT" && ok "detach" || bad "detach"
exec_in "$RT3" "test -f /mnt/hot/h.txt" >/dev/null 2>&1 \
  && bad "hot.txt still visible after detach" || ok "unmounted"

# lifecycle: pause/resume then kill-detach
gxj runtime pause "$RT1" && wait_status "$RT1" paused && ok "paused" || bad "pause"
gxj runtime resume "$RT1" && wait_status "$RT1" running && ok "resumed" || bad "resume"
exec_in "$RT1" "cat /mnt/bucket/p2.txt" | grep -q "p2-$RUN_ID" \
  && ok "mount survives pause/resume" || bad "mount dead after resume"

gxj runtime kill "$RT1" >/dev/null && RUNTIMES=("${RUNTIMES[@]/$RT1/}")
end=$((SECONDS + 90)); DETACHED=0
while (( SECONDS < end )); do
  LEFT="$(gxj runtime mounts list "$RT1" 2>/dev/null |
    jq -r '[.mounts[]? | select(.state=="attached")] | length')"
  [[ "$LEFT" == "0" ]] && { DETACHED=1; break; }
  sleep 3
done
(( DETACHED )) && ok "kill detaches" || bad "attachments still attached after kill"

# ------------------------------------------------------------- bucket link ----

if [[ -n "${AWS_ACCESS_KEY_ID:-}" && -n "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
  echo "== bucket link (static creds) =="
  PROV="$(gxj provider create "e2e-aws-$RUN_ID" \
    --secret "AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID" \
    --secret "AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY" | jq -r '.id')"
  PROVIDERS+=("$PROV")
  [[ -n "$PROV" && "$PROV" != null ]] && ok "provider create" || bad "provider create"

  LINK="$(gxj storage link create "e2e-link-$RUN_ID" "$TEST_BUCKET" \
    --provider s3 --region "$TEST_REGION" --auth-mode static \
    --secret-provider-id "$PROV" \
    --access-key-secret AWS_ACCESS_KEY_ID \
    --secret-key-secret AWS_SECRET_ACCESS_KEY | jq -r '.id')"
  LINKS+=("$LINK")
  [[ -n "$LINK" && "$LINK" != null ]] && ok "link create" || bad "link create"
  [[ "$(gxj storage link verify "$LINK" | jq -r '.status')" == "verified" ]] \
    && ok "link verify" || bad "link verify"

  RT4="$(gxj runtime create --template "$TEMPLATE" --wait \
    --mount "{\"kind\":\"bucket\",\"path\":\"/mnt/byo\",\"link\":\"$LINK\",\"mode\":\"rw\"}" \
    | jq -r '.runtime_id // .id')"
  RUNTIMES+=("$RT4")
  wait_status "$RT4" running && wait_mount "$RT4" /mnt/byo \
    && ok "link mount attached" || bad "link mount"
  exec_in "$RT4" "echo via-link-$RUN_ID > /mnt/byo/v.txt && sync && cat /mnt/byo/v.txt" \
    | grep -q "via-link-$RUN_ID" && ok "link io" || bad "link io"
else
  echo "== bucket link SKIPPED (no AWS creds) =="
fi

# -------------------------------------------------------------------- git ----

if [[ -n "${PUBLIC_REPO:-}" ]]; then
  echo "== git mount (public) =="
  RT5="$(gxj runtime create --template "$TEMPLATE" --wait \
    --mount "{\"kind\":\"git\",\"path\":\"/mnt/repo\",\"url\":\"$PUBLIC_REPO\",\"depth\":1}" \
    | jq -r '.runtime_id // .id')"
  RUNTIMES+=("$RT5")
  wait_status "$RT5" running && wait_mount "$RT5" /mnt/repo \
    && ok "git attached" || bad "git attach"
  exec_in "$RT5" "ls /mnt/repo | head -3" >/dev/null \
    && ok "git tree readable" || bad "git tree empty"
  exec_in "$RT5" "test -d /mnt/repo/.git" >/dev/null 2>&1 \
    && bad ".git materialized" || ok ".git excluded"
  exec_in "$RT5" "touch /mnt/repo/w.txt" >/dev/null 2>&1 \
    && bad "git mount writable" || ok "git read-only"
else
  echo "== git mount SKIPPED (no PUBLIC_REPO) =="
fi

echo
echo "=============================================="
echo "storage e2e: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
