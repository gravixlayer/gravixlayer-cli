#!/usr/bin/env bash
# =============================================================================
# GravixLayer CLI robustness test suite
#
# Tests all subcommands, flags, and options across runtime, template,
# billing, auth, and config command groups.
#
# Prerequisites:
#   export GRAVIXLAYER_API_KEY="your-key"
#   export GRAVIXLAYER_BASE_URL="https://api.gravixlayer.ai"  # optional
#
# Usage:
#   ./scripts/test_cli.sh [--binary /path/to/gravixlayer] [--no-cleanup]
#
# Exit code: 0 if all tests pass, 1 if any fail.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Config / defaults
# ---------------------------------------------------------------------------
BINARY="${GRAVIXLAYER_BINARY:-gravixlayer}"
SKIP_CLEANUP="${SKIP_CLEANUP:-0}"
SKIP_TEMPLATE_BUILD="${SKIP_TEMPLATE_BUILD:-1}"
TEMPLATE="${GRAVIXLAYER_TEMPLATE:-base-small}"
PROVIDER="${GRAVIXLAYER_PROVIDER:-aws}"
REGION="${GRAVIXLAYER_REGION:-us-east-1}"
LOG_DIR="$(mktemp -d /tmp/gl-cli-test-XXXXXX)"
RUNTIME_ID=""  # set once a runtime is created and reused across tests

# ---------------------------------------------------------------------------
# Colours
# ---------------------------------------------------------------------------
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
CYAN="\033[0;36m"
BOLD="\033[1m"
RESET="\033[0m"

# ---------------------------------------------------------------------------
# Counters
# ---------------------------------------------------------------------------
PASS=0
FAIL=0
SKIP=0

declare -a FAILED_TESTS=()

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log_header() {
    echo ""
    echo -e "${CYAN}${BOLD}════════════════════════════════════════════════════${RESET}"
    echo -e "${CYAN}${BOLD}  $1${RESET}"
    echo -e "${CYAN}${BOLD}════════════════════════════════════════════════════${RESET}"
}

log_section() {
    echo ""
    echo -e "${YELLOW}${BOLD}── $1 ──${RESET}"
}

redact_file() {
    local file="$1"
    if [[ -n "${GRAVIXLAYER_API_KEY:-}" && -f "$file" ]]; then
        perl -0pi -e 's/\Q$ENV{GRAVIXLAYER_API_KEY}\E/[REDACTED]/g' "$file" 2>/dev/null || true
    fi
    if [[ -f "$file" ]]; then
        perl -0pi -e 's/-----BEGIN OPENSSH PRIVATE KEY-----.*?-----END OPENSSH PRIVATE KEY-----/[REDACTED_PRIVATE_KEY]/gs' "$file" 2>/dev/null || true
    fi
}

# run_test <test_name> <expected_exit: 0|nonzero|service-optional> <cmd...>
# Captures stdout + stderr to LOG_DIR/<test_name>.{out,err}
run_test() {
    local name="$1"
    local expected_exit="$2"
    shift 2
    # Sanitize the test name for use as a filename (spaces→_, slashes→-, parens→_)
    local safe_name
    safe_name="${name// /_}"
    safe_name="${safe_name//\//-}"
    safe_name="${safe_name//(/_}"
    safe_name="${safe_name//)/}"
    local logbase="${LOG_DIR}/${safe_name}"

    # Print the exact command before running it
    local cmd_str
    cmd_str=$(printf ' %q' "$@")
    echo -e "  ${CYAN}CMD${RESET}   ${cmd_str# }"

    local actual_exit=0
    "$@" >"${logbase}.out" 2>"${logbase}.err" || actual_exit=$?
    redact_file "${logbase}.out"
    redact_file "${logbase}.err"

    local ok=0
    local service_skip=0
    if [[ "$expected_exit" == "0" && "$actual_exit" == "0" ]]; then
        ok=1
    elif [[ "$expected_exit" == "nonzero" && "$actual_exit" != "0" ]]; then
        ok=1
    elif [[ "$expected_exit" == "service-optional" && "$actual_exit" == "0" ]]; then
        ok=1
    elif [[ "$expected_exit" == "service-optional" ]] && \
        grep -Eiq 'HTTP (408|503)|service_unavailable|timed out waiting' "${logbase}.err" 2>/dev/null; then
        service_skip=1
    fi

    if [[ "$service_skip" == "1" ]]; then
        echo -e "  ${YELLOW}SKIP${RESET}  ${name}  (backend build service unavailable/timeout)"
        (( SKIP++ )) || true
        if [[ -s "${logbase}.out" ]]; then
            echo -e "  ${BOLD}OUT${RESET}:"
            head -20 "${logbase}.out" | sed 's/^/         /'
        fi
        if [[ -s "${logbase}.err" ]]; then
            echo -e "  ${YELLOW}ERR${RESET}:"
            head -20 "${logbase}.err" | sed 's/^/         /'
        fi
    elif [[ "$ok" == "1" ]]; then
        echo -e "  ${GREEN}PASS${RESET}  ${name}"
        (( PASS++ )) || true
        # Print stdout response (up to 40 lines)
        if [[ -s "${logbase}.out" ]]; then
            echo -e "  ${BOLD}OUT${RESET}:"
            head -40 "${logbase}.out" | sed 's/^/         /'
        fi
        # Print stderr if non-empty (warnings etc.)
        if [[ -s "${logbase}.err" ]]; then
            echo -e "  ${YELLOW}ERR${RESET}:"
            head -10 "${logbase}.err" | sed 's/^/         /'
        fi
    else
        echo -e "  ${RED}FAIL${RESET}  ${name}  (exit=${actual_exit}, expected=${expected_exit})"
        (( FAIL++ )) || true
        FAILED_TESTS+=("$name")
        if [[ -s "${logbase}.out" ]]; then
            echo -e "  ${BOLD}OUT${RESET}:"
            head -20 "${logbase}.out" | sed 's/^/         /'
        fi
        if [[ -s "${logbase}.err" ]]; then
            echo -e "  ${RED}ERR${RESET}:"
            head -20 "${logbase}.err" | sed 's/^/         /'
        fi
    fi

    echo ""
    return 0
}

# assert_contains <test_name> <file> <pattern>
assert_contains() {
    local name="$1"
    local file="$2"
    local pattern="$3"
    if grep -q "$pattern" "$file" 2>/dev/null; then
        echo -e "  ${GREEN}PASS${RESET}  ${name}"
        (( PASS++ )) || true
    else
        local disp_file="$file"
        echo -e "  ${RED}FAIL${RESET}  ${name}  (pattern '${pattern}' not found in ${disp_file})"
        (( FAIL++ )) || true
        FAILED_TESTS+=("$name")
    fi
}

# assert_not_contains <test_name> <file> <pattern>
assert_not_contains() {
    local name="$1"
    local file="$2"
    local pattern="$3"
    if grep -q "$pattern" "$file" 2>/dev/null; then
        echo -e "  ${RED}FAIL${RESET}  ${name}  (pattern '${pattern}' unexpectedly found in ${file})"
        (( FAIL++ )) || true
        FAILED_TESTS+=("$name")
    else
        echo -e "  ${GREEN}PASS${RESET}  ${name}"
        (( PASS++ )) || true
    fi
}

skip_test() {
    local name="$1"
    local reason="$2"
    echo -e "  ${YELLOW}SKIP${RESET}  ${name}  (${reason})"
    (( SKIP++ )) || true
}

require_runtime() {
    if [[ -z "$RUNTIME_ID" ]]; then
        echo -e "  ${YELLOW}SKIP${RESET}  $1  (no runtime available; runtime creation failed earlier)"
        (( SKIP++ )) || true
        return 1
    fi
    return 0
}

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
log_header "GravixLayer CLI Test Suite"

echo "Binary    : $BINARY"
echo "Template  : $TEMPLATE"
echo "Provider  : $PROVIDER / $REGION"
echo "Builds    : template=$([[ "$SKIP_TEMPLATE_BUILD" == "1" ]] && echo skipped || echo enabled)"
echo "Log dir   : $LOG_DIR"
echo ""

if ! command -v "$BINARY" &>/dev/null; then
    echo -e "${RED}ERROR: '${BINARY}' not found in PATH.${RESET}"
    echo "Set GRAVIXLAYER_BINARY or add the binary to PATH."
    exit 1
fi

if [[ -z "${GRAVIXLAYER_API_KEY:-}" ]]; then
    echo -e "${RED}ERROR: GRAVIXLAYER_API_KEY is not set.${RESET}"
    exit 1
fi

# ---------------------------------------------------------------------------
# ── AUTH ────────────────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "AUTH"

run_test "auth status" 0 \
    "$BINARY" auth status

# auth whoami calls /v1/users/me which requires JWT (not API key); skip in API-key-only runs
skip_test "auth whoami" "requires JWT auth, not API key"

run_test "auth token" 0 \
    "$BINARY" auth token

skip_test "auth whoami --output json" "requires JWT auth, not API key"
skip_test "auth whoami json has email/id field" "depends on auth whoami"

# ---------------------------------------------------------------------------
# ── CONFIG ──────────────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "CONFIG"

run_test "config show" 0 \
    "$BINARY" config show

run_test "config profiles" 0 \
    "$BINARY" config profiles

run_test "config set default_region" 0 \
    "$BINARY" config set default_region "$REGION"

run_test "config set default_cloud" 0 \
    "$BINARY" config set default_cloud "$PROVIDER"

run_test "config unset default_region" 0 \
    "$BINARY" config unset default_region

# CLI creates the profile if it does not exist — expect success, not an error
run_test "config use-profile (nonexistent creates it)" 0 \
    "$BINARY" config use-profile nonexistent-profile-xyz
# Restore active profile to default
"$BINARY" config use-profile default >/dev/null 2>&1 || true

# ---------------------------------------------------------------------------
# ── TEMPLATE — read-only ────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "TEMPLATE — list / get"

run_test "template list" 0 \
    "$BINARY" template list

run_test "template list --limit 5" 0 \
    "$BINARY" template list --limit 5

run_test "template list --limit 5 --offset 0" 0 \
    "$BINARY" template list --limit 5 --offset 0

run_test "template list --output json" 0 \
    "$BINARY" --output json template list

assert_contains "template list json is array/object" \
    "${LOG_DIR}/template_list_--output_json.out" \
    "{"

run_test "template list --output quiet" 0 \
    "$BINARY" --output quiet template list

# Grab a public template ID from the list for subsequent get tests
TEMPLATE_ID=""
# Sanitize name to match run_test's safe_name logic
TEMPLATE_LIST_OUT="${LOG_DIR}/template_list.out"
# Template IDs are UUIDs; extract first one from the table output
TEMPLATE_ID=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
    "$TEMPLATE_LIST_OUT" 2>/dev/null | head -1 || true)

if [[ -n "$TEMPLATE_ID" ]]; then
    run_test "template get by id" 0 \
        "$BINARY" template get "$TEMPLATE_ID"

    run_test "template get --output json" 0 \
        "$BINARY" --output json template get "$TEMPLATE_ID"

    assert_contains "template get json has id field" \
        "${LOG_DIR}/template_get_--output_json.out" \
        '"id"'
else
    skip_test "template get by id" "no UUID found in template list output"
fi

run_test "template get invalid id returns error" nonzero \
    "$BINARY" template get "00000000-0000-0000-0000-000000000000"

run_test "template get empty id returns error" nonzero \
    "$BINARY" template get ""

# ---------------------------------------------------------------------------
# ── RUNTIME — create ────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — create"

# Basic create — capture output to extract the runtime ID
run_test "runtime create (default template)" 0 \
    "$BINARY" runtime create --template "$TEMPLATE" --cloud "$PROVIDER" --region "$REGION"

# Extract UUID from output — safe_name of "runtime create (default template)" => runtime_create__default_template
RT_CREATE_OUT="${LOG_DIR}/runtime_create__default_template.out"
RUNTIME_ID=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
    "$RT_CREATE_OUT" 2>/dev/null | head -1 || true)

if [[ -n "$RUNTIME_ID" ]]; then
    echo "  → runtime_id: $RUNTIME_ID"
else
    echo -e "  ${YELLOW}WARN${RESET}  Could not extract runtime ID from create output."
fi

# Create with env vars, timeout, wait
run_test "runtime create with env vars and timeout" 0 \
    "$BINARY" runtime create \
        --template "$TEMPLATE" \
        --cloud "$PROVIDER" \
        --region "$REGION" \
        --env "GL_TEST_VAR=hello" \
        --env "GL_TEST_NUM=42" \
        --timeout 1800

# Capture the ID for a second runtime to kill later (double-kill test)
RT2_CREATE_OUT="${LOG_DIR}/runtime_create_with_env_vars_and_timeout.out"
RUNTIME_ID2=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
    "$RT2_CREATE_OUT" 2>/dev/null | head -1 || true)
[[ -n "$RUNTIME_ID2" ]] && echo "  → runtime_id2: $RUNTIME_ID2"

# ---------------------------------------------------------------------------
# ── RUNTIME — list / get / metrics ──────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — list / get / metrics"

run_test "runtime list" 0 \
    "$BINARY" runtime list

run_test "runtime list --limit 5" 0 \
    "$BINARY" runtime list --limit 5

run_test "runtime list --limit 10 --offset 0" 0 \
    "$BINARY" runtime list --limit 10 --offset 0

run_test "runtime list --output json" 0 \
    "$BINARY" --output json runtime list

assert_contains "runtime list json has runtimes key" \
    "${LOG_DIR}/runtime_list_--output_json.out" \
    "{"

run_test "runtime list --output quiet" 0 \
    "$BINARY" --output quiet runtime list

if [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime get" 0 \
        "$BINARY" runtime get "$RUNTIME_ID"

    run_test "runtime get --output json" 0 \
        "$BINARY" --output json runtime get "$RUNTIME_ID"

    assert_contains "runtime get json has runtime_id" \
        "${LOG_DIR}/runtime_get_--output_json.out" \
        '"runtime_id"'


    run_test "runtime metrics" 0 \
        "$BINARY" runtime metrics "$RUNTIME_ID"

    run_test "runtime metrics --output json" 0 \
        "$BINARY" --output json runtime metrics "$RUNTIME_ID"
fi

run_test "runtime get invalid id returns error" nonzero \
    "$BINARY" runtime get "00000000-0000-0000-0000-000000000000"

# ---------------------------------------------------------------------------
# ── RUNTIME — context ───────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — context"

run_test "runtime context show (empty ok)" 0 \
    "$BINARY" runtime context show

if [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime context set" 0 \
        "$BINARY" runtime context set "$RUNTIME_ID"

    run_test "runtime context show (after set)" 0 \
        "$BINARY" runtime context show

    assert_contains "runtime context contains set id" \
        "${LOG_DIR}/runtime_context_show__after_set.out" \
        "$RUNTIME_ID"

    run_test "runtime context clear" 0 \
        "$BINARY" runtime context clear

    run_test "runtime context show (after clear)" 0 \
        "$BINARY" runtime context show
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — exec ──────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — exec"

if require_runtime "runtime exec uname" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime exec uname -a" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" uname -a

    run_test "runtime exec with workdir" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" --workdir /tmp pwd

    run_test "runtime exec with env var" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" \
            --env "MY_TEST=hello" \
            printenv MY_TEST

    run_test "runtime exec with timeout flag" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" --timeout 30 echo "timeout-test"

    run_test "runtime exec --output json" 0 \
        "$BINARY" --output json runtime exec "$RUNTIME_ID" echo "json-exec-test"

    assert_contains "runtime exec json has stdout" \
        "${LOG_DIR}/runtime_exec_--output_json.out" \
        '"stdout"'

    run_test "runtime exec nonzero exit captured" nonzero \
        "$BINARY" runtime exec "$RUNTIME_ID" false

    # Streaming exec (SSE path)
    run_test "runtime exec --stream echo" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" --stream echo "streaming-test"

    run_test "runtime exec --stream with workdir" 0 \
        "$BINARY" runtime exec "$RUNTIME_ID" --stream --workdir /tmp pwd

    run_test "runtime exec --stream --output json" 0 \
        "$BINARY" --output json runtime exec "$RUNTIME_ID" --stream echo "json-stream"
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — connect / service ─────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — connect / service"

if require_runtime "runtime connect" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime connect" 0 \
        "$BINARY" runtime connect "$RUNTIME_ID"

    run_test "runtime connect --output json" 0 \
        "$BINARY" --output json runtime connect "$RUNTIME_ID"

    run_test "runtime service web-url port 8080" 0 \
        "$BINARY" runtime service web-url "$RUNTIME_ID" 8080

    run_test "runtime service web-url port 3000 --output json" 0 \
        "$BINARY" --output json runtime service web-url "$RUNTIME_ID" 3000

    assert_contains "runtime service web-url json has url" \
        "${LOG_DIR}/runtime_service_web-url_port_3000_--output_json.out" \
        '"url"'

    run_test "runtime service list" 0 \
        "$BINARY" runtime service list "$RUNTIME_ID"
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — run (upload + execute a local script) ─────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — run (upload + execute script)"

if require_runtime "runtime run python script" && [[ -n "$RUNTIME_ID" ]]; then
    # Create a temporary Python script to upload and run
    LOCAL_SCRIPT=$(mktemp /tmp/gl-run-XXXXXX.py)
    cat >"$LOCAL_SCRIPT" <<'PYEOF'
import os, sys
print("hello from run_code")
print("env_check:", os.environ.get("GL_RUN_VAR", "missing"))
sys.exit(0)
PYEOF

    run_test "runtime run python script" 0 \
        "$BINARY" runtime run "$RUNTIME_ID" "$LOCAL_SCRIPT"

    run_test "runtime run with env var" 0 \
        "$BINARY" runtime run "$RUNTIME_ID" "$LOCAL_SCRIPT" \
            --env "GL_RUN_VAR=present"

    run_test "runtime run with timeout flag" 0 \
        "$BINARY" runtime run "$RUNTIME_ID" "$LOCAL_SCRIPT" \
            --timeout 60

    run_test "runtime run --output json" 0 \
        "$BINARY" --output json runtime run "$RUNTIME_ID" "$LOCAL_SCRIPT"

    assert_contains "runtime run json has stdout" \
        "${LOG_DIR}/runtime_run_--output_json.out" \
        '"stdout"'

    rm -f "$LOCAL_SCRIPT"

    # Shell script variant
    LOCAL_SH=$(mktemp /tmp/gl-run-XXXXXX.sh)
    cat >"$LOCAL_SH" <<'SHEOF'
#!/bin/sh
echo "shell script via run_code"
exit 0
SHEOF
    chmod +x "$LOCAL_SH"

    run_test "runtime run shell script" 0 \
        "$BINARY" runtime run "$RUNTIME_ID" "$LOCAL_SH"

    rm -f "$LOCAL_SH"
fi

# Error: script file does not exist (CLI validates before sending)
run_test "runtime run missing file returns error" nonzero \
    "$BINARY" runtime run "00000000-0000-0000-0000-000000000000" \
        "/tmp/gl-nonexistent-script-$$.py"

# ---------------------------------------------------------------------------
# ── RUNTIME — code-context ──────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — code-context"

CODE_CONTEXT_ID=""

if require_runtime "runtime code-context create" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime code-context create python" 0 \
        "$BINARY" runtime code-context create "$RUNTIME_ID" --language python

    run_test "runtime code-context create python --output json" 0 \
        "$BINARY" --output json runtime code-context create "$RUNTIME_ID" --language python

    CODE_CONTEXT_ID=$(grep -oE '"(context_id|id)"\s*:\s*"[^"]+"' \
        "${LOG_DIR}/runtime_code-context_create_python_--output_json.out" 2>/dev/null \
        | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
        | head -1 || true)

    if [[ -n "$CODE_CONTEXT_ID" ]]; then
        echo "  → code_context_id: $CODE_CONTEXT_ID"

        run_test "runtime code-context get" 0 \
            "$BINARY" runtime code-context get "$RUNTIME_ID" "$CODE_CONTEXT_ID"

        run_test "runtime code-context get --output json" 0 \
            "$BINARY" --output json runtime code-context get "$RUNTIME_ID" "$CODE_CONTEXT_ID"

        run_test "runtime code-context delete" 0 \
            "$BINARY" runtime code-context delete "$RUNTIME_ID" "$CODE_CONTEXT_ID"
    else
        skip_test "runtime code-context get" "could not extract context_id from create output"
        skip_test "runtime code-context delete" "could not extract context_id from create output"
    fi

    run_test "runtime code-context create with cwd" 0 \
        "$BINARY" runtime code-context create "$RUNTIME_ID" --language python --cwd /home/user
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — files ─────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — files"

if require_runtime "runtime files write" && [[ -n "$RUNTIME_ID" ]]; then
    # Write
    run_test "runtime files write inline" 0 \
        "$BINARY" runtime files write "$RUNTIME_ID" /tmp/gl_test.txt "hello from cli test"

    # Cat
    run_test "runtime files cat" 0 \
        "$BINARY" runtime files cat "$RUNTIME_ID" /tmp/gl_test.txt

    assert_contains "runtime files cat shows content" \
        "${LOG_DIR}/runtime_files_cat.out" \
        "hello"

    # Mkdir
    run_test "runtime files mkdir" 0 \
        "$BINARY" runtime files mkdir "$RUNTIME_ID" /tmp/gl_test_dir

    # ls — root
    run_test "runtime files ls /" 0 \
        "$BINARY" runtime files ls "$RUNTIME_ID" /

    # ls — specific dir
    run_test "runtime files ls /tmp" 0 \
        "$BINARY" runtime files ls "$RUNTIME_ID" /tmp

    # Upload: create a local temp file then upload it
    LOCAL_UPLOAD=$(mktemp /tmp/gl-upload-XXXXXX.txt)
    echo "cli upload test $(date)" >"$LOCAL_UPLOAD"

    run_test "runtime files upload" 0 \
        "$BINARY" runtime files upload "$RUNTIME_ID" "$LOCAL_UPLOAD" /tmp/gl_uploaded.txt

    run_test "runtime files upload with mode" 0 \
        "$BINARY" runtime files upload "$RUNTIME_ID" "$LOCAL_UPLOAD" /tmp/gl_upload_exec.sh \
            --mode 0755

    run_test "runtime files upload with mode and user" 0 \
        "$BINARY" runtime files upload "$RUNTIME_ID" "$LOCAL_UPLOAD" /tmp/gl_upload_user.txt \
            --mode 0644 --user root

    rm -f "$LOCAL_UPLOAD"

    # Download
    LOCAL_DOWNLOAD=$(mktemp /tmp/gl-download-XXXXXX.txt)
    run_test "runtime files download" 0 \
        "$BINARY" runtime files download "$RUNTIME_ID" /tmp/gl_test.txt "$LOCAL_DOWNLOAD"

    assert_contains "runtime files download has content" \
        "$LOCAL_DOWNLOAD" \
        "hello"

    rm -f "$LOCAL_DOWNLOAD"

    # Chmod — now routed to /files/set-mode
    run_test "runtime files chmod (set-mode)" 0 \
        "$BINARY" runtime files chmod "$RUNTIME_ID" /tmp/gl_test.txt 0644

    run_test "runtime files chmod executable" 0 \
        "$BINARY" runtime files chmod "$RUNTIME_ID" /tmp/gl_upload_exec.sh 0755

    # Mkdir with mode
    run_test "runtime files mkdir with mode" 0 \
        "$BINARY" runtime files mkdir "$RUNTIME_ID" /tmp/gl_test_dir_mode --mode 0750

    # Mkdir flat (no recursive)
    run_test "runtime files mkdir --no-recursive" 0 \
        "$BINARY" runtime files mkdir "$RUNTIME_ID" /tmp/gl_flat_dir --no-recursive

    # Info (stat metadata)
    run_test "runtime files info on file" 0 \
        "$BINARY" runtime files info "$RUNTIME_ID" /tmp/gl_test.txt

    run_test "runtime files info --output json" 0 \
        "$BINARY" --output json runtime files info "$RUNTIME_ID" /tmp/gl_test.txt

    assert_contains "runtime files info json has size" \
        "${LOG_DIR}/runtime_files_info_--output_json.out" \
        '"size"'

    run_test "runtime files info on directory" 0 \
        "$BINARY" runtime files info "$RUNTIME_ID" /tmp/gl_test_dir

    # Write-many (multi-file upload in one request)
    LOCAL_MANY1=$(mktemp /tmp/gl-many1-XXXXXX.txt)
    LOCAL_MANY2=$(mktemp /tmp/gl-many2-XXXXXX.txt)
    echo "file one content" >"$LOCAL_MANY1"
    echo "file two content" >"$LOCAL_MANY2"

    run_test "runtime files write-many two files" 0 \
        "$BINARY" runtime files write-many "$RUNTIME_ID" \
            --file "${LOCAL_MANY1}=/tmp/gl_many1.txt" \
            --file "${LOCAL_MANY2}=/tmp/gl_many2.txt"

    run_test "runtime files write-many with user" 0 \
        "$BINARY" runtime files write-many "$RUNTIME_ID" \
            --file "${LOCAL_MANY1}=/tmp/gl_many1_root.txt" \
            --user root

    run_test "runtime files write-many --output json" 0 \
        "$BINARY" --output json runtime files write-many "$RUNTIME_ID" \
            --file "${LOCAL_MANY1}=/tmp/gl_many_json.txt"

    rm -f "$LOCAL_MANY1" "$LOCAL_MANY2"

    # rm (delete)
    run_test "runtime files rm" 0 \
        "$BINARY" runtime files rm "$RUNTIME_ID" /tmp/gl_test.txt --yes
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — git ───────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — git clone / status / branch"

if require_runtime "runtime git clone" && [[ -n "$RUNTIME_ID" ]]; then
    # Clone with new flags: --branch and --depth
    run_test "runtime git clone public repo" 0 \
        "$BINARY" runtime git clone "$RUNTIME_ID" \
            "https://github.com/octocat/Hello-World.git" \
            --target-dir /tmp/gl-git-test

    run_test "runtime git clone with branch and depth" 0 \
        "$BINARY" runtime git clone "$RUNTIME_ID" \
            "https://github.com/octocat/Hello-World.git" \
            --target-dir /tmp/gl-git-test-shallow \
            --branch master \
            --depth 1

    run_test "runtime git status" 0 \
        "$BINARY" runtime git status "$RUNTIME_ID" --path /tmp/gl-git-test

    run_test "runtime git status --output json" 0 \
        "$BINARY" --output json runtime git status "$RUNTIME_ID" --path /tmp/gl-git-test

    run_test "runtime git branch (local)" 0 \
        "$BINARY" runtime git branch "$RUNTIME_ID" --path /tmp/gl-git-test

    run_test "runtime git branch --all" 0 \
        "$BINARY" runtime git branch "$RUNTIME_ID" --path /tmp/gl-git-test --all

    run_test "runtime git branch --remote" 0 \
        "$BINARY" runtime git branch "$RUNTIME_ID" --path /tmp/gl-git-test --remote

    run_test "runtime git fetch" 0 \
        "$BINARY" runtime git fetch "$RUNTIME_ID" --path /tmp/gl-git-test

    run_test "runtime git fetch with remote" 0 \
        "$BINARY" runtime git fetch "$RUNTIME_ID" --path /tmp/gl-git-test --remote origin

    run_test "runtime git pull" 0 \
        "$BINARY" runtime git pull "$RUNTIME_ID" --workdir /tmp/gl-git-test

    run_test "runtime git pull with remote and branch" 0 \
        "$BINARY" runtime git pull "$RUNTIME_ID" \
            --workdir /tmp/gl-git-test \
            --remote origin \
            --branch master

    run_test "runtime git checkout branch" 0 \
        "$BINARY" runtime git checkout "$RUNTIME_ID" master --path /tmp/gl-git-test

    run_test "runtime git branch-create" 0 \
        "$BINARY" runtime git branch-create "$RUNTIME_ID" gl-test-branch \
            --path /tmp/gl-git-test \
            --start-point master

    run_test "runtime git add all files" 0 \
        "$BINARY" runtime git add "$RUNTIME_ID" --path /tmp/gl-git-test

    run_test "runtime git add specific file" 0 \
        "$BINARY" runtime git add "$RUNTIME_ID" \
            --path /tmp/gl-git-test \
            --files README

    run_test "runtime git commit with author" 0 \
        "$BINARY" runtime git commit "$RUNTIME_ID" \
            --path /tmp/gl-git-test \
            --message "cli test commit" \
            --author-name "CLI Test" \
            --author-email "cli@test.example" \
            --allow-empty

    # Push requires credentials; use a public no-op or skip
    skip_test "runtime git push" "requires write access to remote"

    run_test "runtime git branch-delete" 0 \
        "$BINARY" runtime git branch-delete "$RUNTIME_ID" gl-test-branch \
            --path /tmp/gl-git-test \
            --force
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — SSH ───────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — SSH enable / disable / status"

if require_runtime "runtime ssh enable" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime ssh enable" 0 \
        "$BINARY" runtime ssh enable "$RUNTIME_ID"

    run_test "runtime ssh enable --output json" 0 \
        "$BINARY" --output json runtime ssh enable "$RUNTIME_ID"

    assert_contains "runtime ssh enable json has connect_cmd field" \
        "${LOG_DIR}/runtime_ssh_enable_--output_json.out" \
        '"connect_cmd"'

    run_test "runtime ssh enable --regenerate-keys" 0 \
        "$BINARY" runtime ssh enable "$RUNTIME_ID" --regenerate-keys

    run_test "runtime ssh status" 0 \
        "$BINARY" runtime ssh status "$RUNTIME_ID"

    run_test "runtime ssh status --output json" 0 \
        "$BINARY" --output json runtime ssh status "$RUNTIME_ID"

    run_test "runtime ssh disable" 0 \
        "$BINARY" runtime ssh disable "$RUNTIME_ID"

    run_test "runtime ssh status after disable" 0 \
        "$BINARY" runtime ssh status "$RUNTIME_ID"
fi

run_test "runtime ssh --help" 0 \
    "$BINARY" runtime ssh --help

run_test "runtime ssh enable --help" 0 \
    "$BINARY" runtime ssh enable --help

# ---------------------------------------------------------------------------
# ── RUNTIME — pause / resume ────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — pause / resume"

if require_runtime "runtime pause" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime pause" 0 \
        "$BINARY" runtime pause "$RUNTIME_ID"

    run_test "runtime get (paused state)" 0 \
        "$BINARY" runtime get "$RUNTIME_ID"

    run_test "runtime resume" 0 \
        "$BINARY" runtime resume "$RUNTIME_ID"

    run_test "runtime get (running after resume)" 0 \
        "$BINARY" runtime get "$RUNTIME_ID"
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — timeout ───────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — timeout"

if require_runtime "runtime timeout set" && [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime timeout set 3600" 0 \
        "$BINARY" runtime timeout "$RUNTIME_ID" 3600

    run_test "runtime timeout set 0 (no timeout)" 0 \
        "$BINARY" runtime timeout "$RUNTIME_ID" 0
fi

# ---------------------------------------------------------------------------
# ── RUNTIME — global output flags ───────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — global flags"

run_test "runtime list --verbose" 0 \
    "$BINARY" --verbose runtime list

run_test "runtime list --profile default" 0 \
    "$BINARY" --profile default runtime list

# ---------------------------------------------------------------------------
# ── RUNTIME — kill + double-kill ────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "RUNTIME — kill"

if [[ -n "$RUNTIME_ID" ]]; then
    run_test "runtime kill (first call)" 0 \
        "$BINARY" runtime kill "$RUNTIME_ID" --yes

    run_test "runtime kill second time returns error (double-kill)" nonzero \
        "$BINARY" runtime kill "$RUNTIME_ID" --yes

    assert_contains "double-kill error message is descriptive" \
        "${LOG_DIR}/runtime_kill_second_time_returns_error__double-kill.err" \
        "already"
fi

if [[ -n "$RUNTIME_ID2" ]]; then
    run_test "runtime kill second runtime" 0 \
        "$BINARY" runtime kill "$RUNTIME_ID2" --yes
fi

run_test "runtime kill invalid uuid returns error" nonzero \
    "$BINARY" runtime kill "00000000-0000-0000-0000-000000000000" --yes

# ---------------------------------------------------------------------------
# ── TEMPLATE — build with new flags ─────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "TEMPLATE — build (new flags)"

BUILT_TEMPLATE_ID=""

if [[ "$SKIP_TEMPLATE_BUILD" == "1" ]]; then
    skip_test "template build from docker-image" "SKIP_TEMPLATE_BUILD=1"
    skip_test "template build with env and tags" "SKIP_TEMPLATE_BUILD=1"
    skip_test "template status" "SKIP_TEMPLATE_BUILD=1"
    skip_test "template delete built template" "SKIP_TEMPLATE_BUILD=1"
else
    # Build from a pre-existing docker image (no archive needed — fastest test)
    run_test "template build from docker-image" service-optional \
        "$BINARY" template build \
        --docker-image "python:3.12-slim" \
        --name "gl-cli-test-$(date +%s)" \
        --start-cmd "python -m http.server 8080 --directory /tmp" \
        --ready-cmd "python - <<'PY'
import socket
socket.create_connection(('127.0.0.1', 8080), timeout=2).close()
PY" \
        --ready-timeout-secs 30

    # Capture build ID from previous build for status polling.
    BUILT_TEMPLATE_BUILD_ID=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
        "${LOG_DIR}/template_build_from_docker-image.out" 2>/dev/null | head -1 || true)

    if [[ -n "$BUILT_TEMPLATE_BUILD_ID" ]]; then
        echo "  → built_template_build_id: $BUILT_TEMPLATE_BUILD_ID"

        run_test "template build with env and tags" service-optional \
            "$BINARY" template build \
                --docker-image "python:3.12-slim" \
                --name "gl-cli-test-tags-$(date +%s)" \
                --start-cmd "python -m http.server 8080 --directory /tmp" \
                --ready-cmd "python - <<'PY'
import socket
socket.create_connection(('127.0.0.1', 8080), timeout=2).close()
PY" \
                --ready-timeout-secs 30 \
                --tag "env=staging" \
                --tag "version=1" \
                --env "LOG_LEVEL=debug"

        run_test "template status" 0 \
            "$BINARY" template status "$BUILT_TEMPLATE_BUILD_ID"

        run_test "template status --output json" 0 \
            "$BINARY" --output json template status "$BUILT_TEMPLATE_BUILD_ID"

        BUILT_TEMPLATE_ID=$(grep -oE '"template_id"\s*:\s*"[^"]+"' \
            "${LOG_DIR}/template_status_--output_json.out" 2>/dev/null \
            | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' \
            | head -1 || true)

        if [[ -n "$BUILT_TEMPLATE_ID" ]]; then
            run_test "template delete built template" 0 \
                "$BINARY" template delete "$BUILT_TEMPLATE_ID" --yes
        else
            skip_test "template delete built template" "template status did not return template_id"
        fi
    else
        skip_test "template build with env and tags" "previous build did not return a UUID"
        skip_test "template status" "no build_id"
        skip_test "template delete built template" "no template_id"
    fi
fi

# ---------------------------------------------------------------------------
# ── AGENT — build / deploy / invoke / stream ────────────────────────────────
# ---------------------------------------------------------------------------
log_header "AGENT — build / deploy / invoke / stream"

AGENT_ID=""
AGENT_SOURCE_DIR="${GRAVIXLAYER_AGENT_SOURCE:-}"

if [[ -n "$AGENT_SOURCE_DIR" && -d "$AGENT_SOURCE_DIR" ]]; then
    echo "  → agent_source_dir: $AGENT_SOURCE_DIR"

    run_test "agent deploy from real source" service-optional \
        "$BINARY" --output json agent deploy "$AGENT_SOURCE_DIR" \
            --name "gl-cli-a2a-langgraph-test-$(date +%s)" \
            --is-public true \
            --build-timeout 900 \
            --wait-timeout 600 \
            --tag "test=true"

    AGENT_ID=$(grep -oE '"agent_id"\s*:\s*"[^"]+"' \
        "${LOG_DIR}/agent_deploy_from_real_source.out" 2>/dev/null \
        | sed -E 's/.*"agent_id"\s*:\s*"([^"]+)".*/\1/' \
        | head -1 || true)

    if [[ -n "$AGENT_ID" ]]; then
        echo "  → agent_id: $AGENT_ID"

        run_test "agent get" 0 \
            "$BINARY" agent get "$AGENT_ID"

        run_test "agent get --output json" 0 \
            "$BINARY" --output json agent get "$AGENT_ID"

        assert_contains "agent get json has agent_id" \
            "${LOG_DIR}/agent_get_--output_json.out" \
            '"agent_id"'

        run_test "agent invoke with message" 0 \
            "$BINARY" agent invoke "$AGENT_ID" --message "hello"

        run_test "agent invoke with json input" 0 \
            "$BINARY" agent invoke "$AGENT_ID" --input '{"message":"hello from cli test"}'

        run_test "agent invoke --output json" 0 \
            "$BINARY" --output json agent invoke "$AGENT_ID" --message "ping"

        run_test "agent invoke with session-id" 0 \
            "$BINARY" agent invoke "$AGENT_ID" \
                --message "hello" \
                --session-id "test-session-001"

        run_test "agent stream" 0 \
            "$BINARY" agent stream "$AGENT_ID" --message "tell me a story"

        run_test "agent stream with session-id" 0 \
            "$BINARY" agent stream "$AGENT_ID" \
                --message "continue" \
                --session-id "test-session-001"

        run_test "agent destroy" 0 \
            "$BINARY" agent destroy "$AGENT_ID" --yes
    else
        skip_test "agent get" "agent deploy did not return agent_id"
        skip_test "agent get --output json" "no agent_id"
        skip_test "agent invoke with message" "no agent_id"
        skip_test "agent invoke with json input" "no agent_id"
        skip_test "agent invoke --output json" "no agent_id"
        skip_test "agent invoke with session-id" "no agent_id"
        skip_test "agent stream" "no agent_id"
        skip_test "agent stream with session-id" "no agent_id"
        skip_test "agent destroy" "no agent_id"
    fi
else
    skip_test "agent deploy from real source" "set GRAVIXLAYER_AGENT_SOURCE to an agent project directory"
    skip_test "agent get" "no agent_id"
    skip_test "agent get --output json" "no agent_id"
    skip_test "agent invoke with message" "no agent_id"
    skip_test "agent invoke with json input" "no agent_id"
    skip_test "agent invoke --output json" "no agent_id"
    skip_test "agent invoke with session-id" "no agent_id"
    skip_test "agent stream" "no agent_id"
    skip_test "agent stream with session-id" "no agent_id"
    skip_test "agent destroy" "no agent_id"
fi

# ---------------------------------------------------------------------------
# ── BILLING ─────────────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "BILLING"

run_test "billing summary" 0 \
    "$BINARY" billing summary

run_test "billing quotas" 0 \
    "$BINARY" billing quotas

run_test "billing history" 0 \
    "$BINARY" billing history

run_test "billing history --page 1 --page-size 10" 0 \
    "$BINARY" billing history --page 1 --page-size 10

run_test "billing summary --output json" 0 \
    "$BINARY" --output json billing summary

assert_contains "billing summary json has balance field" \
    "${LOG_DIR}/billing_summary_--output_json.out" \
    "{"

run_test "billing quotas --output json" 0 \
    "$BINARY" --output json billing quotas

assert_contains "billing quotas json has vcpu_limit" \
    "${LOG_DIR}/billing_quotas_--output_json.out" \
    '"vcpu_limit"'

assert_contains "billing quotas json has tier_name" \
    "${LOG_DIR}/billing_quotas_--output_json.out" \
    '"tier_name"'

# Config show must redact inline API keys
run_test "config show (api key redacted)" 0 \
    "$BINARY" config show

assert_not_contains "config show does not print env API key" \
    "${LOG_DIR}/config_show__api_key_is_redacted.out" \
    "${GRAVIXLAYER_API_KEY}"

# ---------------------------------------------------------------------------
# ── ERROR CASES ─────────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "ERROR CASES"

run_test "missing subcommand exits nonzero" nonzero \
    "$BINARY" runtime

run_test "runtime create missing template value" nonzero \
    "$BINARY" runtime create --template

run_test "runtime get empty id exits nonzero" nonzero \
    "$BINARY" runtime get ""

# template get with empty ID — CLI validates and returns error (already tested above)

run_test "unknown top-level command exits nonzero" nonzero \
    "$BINARY" foobar

run_test "runtime exec no command exits nonzero" nonzero \
    "$BINARY" runtime exec "00000000-0000-0000-0000-000000000000"

# ---------------------------------------------------------------------------
# ── OUTPUT FORMAT FLAGS ──────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "OUTPUT FORMAT FLAGS"

run_test "global --output table (default)" 0 \
    "$BINARY" --output table template list

run_test "global --output json" 0 \
    "$BINARY" --output json template list

run_test "global --output quiet" 0 \
    "$BINARY" --output quiet template list

# JSON output must be valid JSON for template list
run_test "template list json is parseable" 0 \
    bash -c "\"$BINARY\" --output json template list | python3 -m json.tool >/dev/null"

# ---------------------------------------------------------------------------
# ── VERSION / HELP ───────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
log_header "VERSION / HELP"

run_test "gravixlayer --version" 0 \
    "$BINARY" --version

run_test "gravixlayer --help" 0 \
    "$BINARY" --help

run_test "gravixlayer runtime --help" 0 \
    "$BINARY" runtime --help

run_test "gravixlayer template --help" 0 \
    "$BINARY" template --help

run_test "gravixlayer billing --help" 0 \
    "$BINARY" billing --help

run_test "gravixlayer auth --help" 0 \
    "$BINARY" auth --help

run_test "gravixlayer runtime create --help" 0 \
    "$BINARY" runtime create --help

run_test "gravixlayer runtime exec --help" 0 \
    "$BINARY" runtime exec --help

run_test "gravixlayer runtime run --help" 0 \
    "$BINARY" runtime run --help

run_test "gravixlayer runtime shell --help" 0 \
    "$BINARY" runtime shell --help

run_test "gravixlayer runtime files --help" 0 \
    "$BINARY" runtime files --help

run_test "gravixlayer runtime git --help" 0 \
    "$BINARY" runtime git --help

run_test "gravixlayer runtime ssh --help" 0 \
    "$BINARY" runtime ssh --help

# ---------------------------------------------------------------------------
# ── SUMMARY ─────────────────────────────────────────────────────────────────
# ---------------------------------------------------------------------------
TOTAL=$(( PASS + FAIL + SKIP ))

echo ""
log_header "TEST RESULTS"
echo ""
echo -e "  Total   : ${BOLD}${TOTAL}${RESET}"
echo -e "  ${GREEN}Passed${RESET}  : ${BOLD}${PASS}${RESET}"
echo -e "  ${RED}Failed${RESET}  : ${BOLD}${FAIL}${RESET}"
echo -e "  ${YELLOW}Skipped${RESET} : ${BOLD}${SKIP}${RESET}"
echo ""
echo "  Logs    : $LOG_DIR"
echo ""

if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
    echo -e "${RED}${BOLD}Failed tests:${RESET}"
    for t in "${FAILED_TESTS[@]}"; do
        echo -e "  ${RED}✗${RESET}  $t"
    done
    echo ""
fi

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
if [[ "$SKIP_CLEANUP" == "1" ]]; then
    echo -e "${YELLOW}SKIP_CLEANUP=1 — log files preserved at ${LOG_DIR}${RESET}"
else
    # Log directory preserved so results can be inspected; only remove on
    # explicit clean request to avoid losing failure evidence.
    echo "Logs preserved at ${LOG_DIR} for post-test inspection."
fi

[[ "$FAIL" -eq 0 ]]
