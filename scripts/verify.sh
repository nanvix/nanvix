#!/bin/bash

# Generic Verus verification script with cheating detection and git tracking.
#
# Two modes:
#   Cargo mode:  ./verify.sh --crate <crate_name> [OPTIONS]
#   Legacy mode: ./verify.sh --verus-dir <dir> [OPTIONS]
#
# Cargo mode uses `cargo verus verify -p <crate>` and is the recommended
# approach for projects using the workspace layout.
#
# Extra environment variables for custom cargo invocations:
#   CARGO_CHANNEL          Rust toolchain channel (e.g., "nightly-2025-12-08").
#   VERUS_EXTRA_CARGO_ARGS Extra arguments appended to the cargo verus verify command.

set -e

# ==============================================================================
# Defaults.
# ==============================================================================

VERUS_DIR=""
CRATE_NAME=""
MODULE=""
VERUS_BIN="${VERUS_BIN:-verus}"
LOG_DIR=""
GIT_COMMIT=true

# ==============================================================================
# Parse arguments.
# ==============================================================================

usage() {
    cat <<EOF
Usage: $(basename "$0") (--crate CRATE | --verus-dir DIR) [OPTIONS]

Mode (pick one):
  --crate CRATE         Cargo crate name (uses: cargo verus verify -p CRATE).
  --verus-dir DIR       Legacy: directory containing lib.rs (uses: verus binary directly).

Options:
  --module MODULE       Module path to verify (e.g., "kernel::pm::thread").
  --verus-bin PATH      Path to verus binary (default: "verus" or \$VERUS_BIN).
  --log-dir DIR         Directory for log files (default: no logging).
  --no-git-commit       Disable auto git-commit of spec/proof files after verification.
  -h, --help            Show this help message.

Environment:
  CARGO_CHANNEL          Toolchain channel for cargo (e.g., "nightly-2025-12-08").
  VERUS_EXTRA_CARGO_ARGS Extra cargo arguments (e.g., "-Z build-std=core,alloc --target ...").
  VERUS_AI_DIR           Path to verus-ai tool directory (for guardrails/tree-sitter).
EOF
    exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --crate)
            CRATE_NAME="$2"
            shift 2
            ;;
        --verus-dir)
            VERUS_DIR="$2"
            shift 2
            ;;
        --module)
            MODULE="$2"
            shift 2
            ;;
        --verus-bin)
            VERUS_BIN="$2"
            shift 2
            ;;
        --log-dir)
            LOG_DIR="$2"
            shift 2
            ;;
        --no-git-commit)
            GIT_COMMIT=false
            shift
            ;;
        -h|--help)
            usage 0
            ;;
        *)
            echo "ERROR: Unknown argument: $1" >&2
            usage 1
            ;;
    esac
done

if [[ -z "$CRATE_NAME" && -z "$VERUS_DIR" ]]; then
    echo "ERROR: Either --crate or --verus-dir is required." >&2
    usage 1
fi

if [[ -n "$CRATE_NAME" && -n "$VERUS_DIR" ]]; then
    echo "ERROR: Use either --crate or --verus-dir, not both." >&2
    usage 1
fi

# ==============================================================================
# Determine verification mode and source directory for cheating detection.
# ==============================================================================

TIMESTAMP="$(date '+%Y-%m-%d_%H-%M-%S')"

if [[ -n "$CRATE_NAME" ]]; then
    # Cargo mode: find the crate's source directory and root file.
    MODE="cargo"
    CRATE_META="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
        | python3 -c "
import sys, json
meta = json.load(sys.stdin)
for pkg in meta['packages']:
    if pkg['name'] == '$CRATE_NAME':
        print(pkg['manifest_path'])
        # Find lib or first target src_path as root file.
        for t in pkg.get('targets', []):
            if 'lib' in t.get('kind', []):
                print(t['src_path'])
                break
        else:
            if pkg.get('targets'):
                print(pkg['targets'][0]['src_path'])
        break
" 2>/dev/null || echo "")"
    CRATE_MANIFEST="$(echo "$CRATE_META" | head -1)"
    CRATE_ROOT_FILE="$(echo "$CRATE_META" | sed -n '2p')"
    if [[ -n "$CRATE_MANIFEST" ]]; then
        CRATE_SRC_DIR="$(dirname "$CRATE_MANIFEST")/src"
    else
        # Fallback: guess common layouts.
        for candidate in "crates/$CRATE_NAME/src" "src/libs/$CRATE_NAME/src" "src/$CRATE_NAME/src"; do
            if [[ -d "$candidate" ]]; then
                CRATE_SRC_DIR="$candidate"
                break
            fi
        done
    fi

    # Build cargo verus command with optional toolchain channel and extra args.
    CARGO_CMD="cargo"
    if [[ -n "${CARGO_CHANNEL:-}" ]]; then
        CARGO_CMD="cargo +${CARGO_CHANNEL}"
    fi

    # shellcheck disable=SC2206
    CMD=($CARGO_CMD verus verify -p "$CRATE_NAME" --no-default-features)

    # Append extra cargo arguments (e.g., -Z build-std, --target).
    if [[ -n "${VERUS_EXTRA_CARGO_ARGS:-}" ]]; then
        # shellcheck disable=SC2206
        CMD+=($VERUS_EXTRA_CARGO_ARGS)
    fi

    if [[ "$MODULE" == "root" ]]; then
        CMD+=(-- --verify-root)
    elif [[ -n "$MODULE" ]]; then
        CMD+=(-- --verify-module "$MODULE")
    fi
    LABEL="${CRATE_NAME}::${MODULE:-all}"

    echo "=== Verus Verification (cargo mode) ==="
    echo "  Crate     : $CRATE_NAME"
    echo "  Module    : ${MODULE:-<all>}"
    echo "  Source dir : ${CRATE_SRC_DIR:-<unknown>}"
    echo "  Channel   : ${CARGO_CHANNEL:-<default>}"
    echo "  Extra args: ${VERUS_EXTRA_CARGO_ARGS:-<none>}"
    echo "  Timestamp : $TIMESTAMP"
else
    # Legacy mode: direct verus binary.
    MODE="legacy"
    if [[ ! -d "$VERUS_DIR" ]]; then
        echo "ERROR: Directory not found: $VERUS_DIR" >&2
        exit 1
    fi
    if [[ ! -f "$VERUS_DIR/lib.rs" ]]; then
        echo "ERROR: lib.rs not found in $VERUS_DIR" >&2
        exit 1
    fi

    CRATE_SRC_DIR="$VERUS_DIR"
    CMD=("$VERUS_BIN" --crate-type lib lib.rs)
    if [[ -n "$MODULE" ]]; then
        CMD+=(--verify-module "$MODULE")
    fi
    LABEL="${MODULE:-all}"

    echo "=== Verus Verification (legacy mode) ==="
    echo "  Directory : $VERUS_DIR"
    echo "  Module    : ${MODULE:-<all>}"
    echo "  Binary    : $VERUS_BIN"
    echo "  Timestamp : $TIMESTAMP"
fi
echo ""

# ==============================================================================
# Run verification.
# ==============================================================================

if [[ "$MODE" == "legacy" ]]; then
    cd "$VERUS_DIR"
fi

# Capture output for logging and analysis.
TMPFILE="$(mktemp)"
trap 'rm -f "$TMPFILE"' EXIT

set +e
set -o pipefail
RUSTC_BOOTSTRAP=1 "${CMD[@]}" 2>&1 | tee "$TMPFILE"
VERUS_EXIT=$?
set +o pipefail
set -e

echo ""

# ==============================================================================
# Parse results.
# ==============================================================================

VERIFIED="$(grep -oP '\d+ verified' "$TMPFILE" | tail -1)"
ERRORS="$(grep -oP '\d+ errors' "$TMPFILE" | tail -1)"

# If no "verification results:" line appeared, the build was cached and Verus
# did not re-verify any crate.  Report this clearly instead of "0 verified".
if [[ -z "$VERIFIED" && -z "$ERRORS" ]]; then
    CACHED_BUILD=true
    VERIFIED="cached (no recompilation)"
    ERRORS="—"
else
    CACHED_BUILD=false
    : "${VERIFIED:=0 verified}"
    : "${ERRORS:=0 errors}"
fi

echo "=== Results ==="
echo "  $VERIFIED"
echo "  $ERRORS"
echo "  Exit code : $VERUS_EXIT"
echo ""

if [[ $VERUS_EXIT -ne 0 ]]; then
    echo "=== Summary (pre-guardrails; verify failed) ==="
    echo "  verification: $VERIFIED, $ERRORS (exit $VERUS_EXIT)"
    echo "  status: VERIFY_FAILED (continuing to guardrail checks for feedback)"
    echo ""
fi

# ==============================================================================
# Cheating pattern detection (tree-sitter AST via guardrails.py).
# ==============================================================================

echo "=== Cheating Pattern Check ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# VERUS_AI_DIR: where guardrails.py and tree-sitter-verus/ live.
# Auto-detected when running from the verus-ai tree; must be set via env var
# when verify.sh is copied into another project.
VERUS_AI_DIR="${VERUS_AI_DIR:-$(dirname "$SCRIPT_DIR")}"

CHEATING_DIR="${CRATE_SRC_DIR:-.}"

# --- Helper: run cheating detection on a directory, print JSON ---
run_cheating_scan() {
    local scan_dir="$1"
    VERUS_AI_DIR="$VERUS_AI_DIR" CHEATING_DIR="$scan_dir" python3 -c '
import sys, os, json
sys.path.insert(0, os.environ["VERUS_AI_DIR"])
from guardrails import detect_cheating
from pathlib import Path

combined = {"assume": 0, "external_body": 0, "admit": 0, "trusted": 0, "no_decreases": 0}
for rs_file in Path(os.environ["CHEATING_DIR"]).glob("**/*.rs"):
    r = detect_cheating(rs_file)
    combined["assume"] += r.assume_count
    combined["external_body"] += r.external_body_count
    combined["admit"] += r.admit_count
    combined["trusted"] += r.trusted_count
    combined["no_decreases"] += r.no_decreases_count
print(json.dumps(combined))
' 2>/dev/null || echo '{"error":"tree-sitter unavailable"}'
}

# --- Helper: get cheating detail (affected functions) for a directory ---
get_cheating_detail() {
    local scan_dir="$1"
    VERUS_AI_DIR="$VERUS_AI_DIR" CHEATING_DIR="$scan_dir" python3 -c '
import sys, os
sys.path.insert(0, os.environ["VERUS_AI_DIR"])
from pathlib import Path
from guardrails import _get_parser, _find_nodes
import re

parser = _get_parser()
for rs_file in sorted(Path(os.environ["CHEATING_DIR"]).glob("**/*.rs")):
    code = rs_file.read_bytes()
    tree = parser.parse(code)
    for fn_node in _find_nodes(tree.root_node, "function_item"):
        name_node = fn_node.child_by_field_name("name")
        if not name_node:
            continue
        fn_name = name_node.text.decode()
        fn_text = fn_node.text.decode()
        attr_text = ""
        sib = fn_node.prev_sibling
        while sib and sib.type == "attribute_item":
            attr_text = sib.text.decode() + "\n" + attr_text
            sib = sib.prev_sibling
        combined = attr_text + fn_text
        issues = []
        if "admit()" in fn_text:
            issues.append("admit")
        if re.search(r"\bassume\s*\(", fn_text):
            issues.append("assume")
        if "external_body" in combined:
            issues.append("external_body")
        if "#[verifier::trusted]" in combined:
            issues.append("trusted")
        if issues:
            line = fn_node.start_point[0] + 1
            rel = rs_file.relative_to(Path(os.environ["CHEATING_DIR"]))
            print(f"    - {rel}:{line} {fn_name}: {", ".join(issues)}")
' 2>/dev/null || true
}

# Count cfg-gated exec code (cheating indicator).
# When MODULE is set, counts only for module files; otherwise whole crate.
count_cfg_gates() {
    local scan_dir="$1"
    python3 -c "
import re, sys
from pathlib import Path

src_dir = Path('$scan_dir')
count = 0
cfg_pat = re.compile(r'#\[cfg(_attr)?\((not\()?verus_keep_ghost')

for rs_file in sorted(src_dir.glob('**/*.rs')):
    lines = rs_file.read_text().splitlines()
    for i, line in enumerate(lines):
        stripped = line.strip()
        if not cfg_pat.search(stripped):
            continue
        if 'derive(' in stripped or 'feature(' in stripped:
            continue
        target = ''
        for j in range(i + 1, min(i + 5, len(lines))):
            t = lines[j].strip()
            if not t or t.startswith('#['):
                continue
            target = t
            break
        if any(target.startswith(k) for k in ['use ', 'use(', 'include!', 'extern ', 'mod ']):
            continue
        if re.match(r'(debug_assert|info|error|warn|trace|debug|log)!\s*\(', target):
            continue
        count += 1

print(count)
" 2>/dev/null
}

# --- Determine module-scoped cheating dir (if MODULE is set) ---
MODULE_CHEATING_DIR=""
if [[ -n "$MODULE" && -n "$CRATE_SRC_DIR" ]]; then
    if [[ "$MODULE" == "root" ]]; then
        MODULE_CHEATING_DIR=""  # root has no separate dir; skip module-scoped
    else
        MOD_CHEAT_PATH="${MODULE//:://}"
        # Collect the module file(s) into a temp dir for scoped scanning
        if [[ -f "$CRATE_SRC_DIR/$MOD_CHEAT_PATH.rs" || -d "$CRATE_SRC_DIR/$MOD_CHEAT_PATH" ]]; then
            MODULE_CHEATING_DIR="$(mktemp -d)"
            trap "rm -rf '$MODULE_CHEATING_DIR'" EXIT
            [[ -f "$CRATE_SRC_DIR/$MOD_CHEAT_PATH.rs" ]] && cp "$CRATE_SRC_DIR/$MOD_CHEAT_PATH.rs" "$MODULE_CHEATING_DIR/"
            if [[ -d "$CRATE_SRC_DIR/$MOD_CHEAT_PATH" ]]; then
                cp -r "$CRATE_SRC_DIR/$MOD_CHEAT_PATH"/* "$MODULE_CHEATING_DIR/" 2>/dev/null || true
            fi
        fi
    fi
fi

# --- Run scans ---
GLOBAL_CHEATING_JSON="$(run_cheating_scan "$CHEATING_DIR")"
GLOBAL_CFG_GATE_COUNT="$(count_cfg_gates "${CRATE_SRC_DIR:-.}")" || GLOBAL_CFG_GATE_COUNT=0

CHEATING_AVAILABLE=false
if echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); sys.exit(0 if 'assume' in d else 1)" 2>/dev/null; then
    CHEATING_AVAILABLE=true
    ASSUME_COUNT="$(echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('assume',0))")"
    EXTERNAL_BODY_COUNT="$(echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('external_body',0))")"
    ADMIT_COUNT="$(echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('admit',0))")"
    TRUSTED_COUNT="$(echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('trusted',0))")"
    NO_DECREASES_COUNT="$(echo "$GLOBAL_CHEATING_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('no_decreases',0))")"
    CFG_GATE_COUNT="$GLOBAL_CFG_GATE_COUNT"
else
    echo "  ⚠️  Cheating detection unavailable (tree-sitter required)."
    echo "     Set VERUS_AI_DIR to the verus-ai tool directory."
    ASSUME_COUNT=0; EXTERNAL_BODY_COUNT=0; ADMIT_COUNT=0; TRUSTED_COUNT=0; NO_DECREASES_COUNT=0
    CFG_GATE_COUNT=0
fi

CHEATING_FOUND=false

if [[ "$CHEATING_AVAILABLE" != "true" ]]; then
    :
elif [[ -n "$MODULE_CHEATING_DIR" ]]; then
    # --- MODULE set: module cheating is primary, global is secondary ---
    MOD_CHEATING_JSON="$(run_cheating_scan "$MODULE_CHEATING_DIR")"
    MOD_CFG_GATE="$(count_cfg_gates "$MODULE_CHEATING_DIR")" || MOD_CFG_GATE=0
    MOD_SUM="$(echo "$MOD_CHEATING_JSON" | python3 -c "
import sys,json; d=json.load(sys.stdin)
print(d.get('assume',0)+d.get('external_body',0)+d.get('admit',0)+d.get('trusted',0)+d.get('no_decreases',0))" 2>/dev/null)" || MOD_SUM=0
    MOD_SUM=$((MOD_SUM + MOD_CFG_GATE))

    if [[ $MOD_SUM -eq 0 ]]; then
        echo "  ✅ No cheating detected in module $MODULE."
    else
        CHEATING_FOUND=true
        echo "  Module $MODULE:"
        echo "$MOD_CHEATING_JSON" | python3 -c "
import sys,json; d=json.load(sys.stdin)
for k in ['assume','external_body','admit','trusted','no_decreases']:
    v=d.get(k,0)
    if v>0: print(f'    ⚠️  {k}: {v}')
" 2>/dev/null
        [[ $MOD_CFG_GATE -gt 0 ]] && echo "    ⚠️  cfg-gated exec code: $MOD_CFG_GATE"
        MOD_DETAIL="$(get_cheating_detail "$MODULE_CHEATING_DIR")"
        [[ -n "$MOD_DETAIL" ]] && echo "  Affected functions:" && echo "$MOD_DETAIL"
    fi

    # Global summary (secondary) — numbers + file
    GLOBAL_TOTAL=$((ASSUME_COUNT + EXTERNAL_BODY_COUNT + ADMIT_COUNT + TRUSTED_COUNT + NO_DECREASES_COUNT + CFG_GATE_COUNT))
    if [[ $GLOBAL_TOTAL -gt 0 ]]; then
        echo "  Global: assume=$ASSUME_COUNT external_body=$EXTERNAL_BODY_COUNT admit=$ADMIT_COUNT trusted=$TRUSTED_COUNT cfg_gate=$CFG_GATE_COUNT"
        CHEATING_DETAIL_FILE="${CRATE_SRC_DIR:-src}/cheating-detail.txt"
        get_cheating_detail "$CHEATING_DIR" > "$CHEATING_DETAIL_FILE"
        echo "  Detail: $CHEATING_DETAIL_FILE"
    fi
elif [[ $((ASSUME_COUNT + EXTERNAL_BODY_COUNT + ADMIT_COUNT + TRUSTED_COUNT + NO_DECREASES_COUNT + CFG_GATE_COUNT)) -eq 0 ]]; then
    echo "  ✅ No cheating detected."
else
    # --- No MODULE: global cheating, write detail to file ---
    CHEATING_FOUND=true
    echo "  cheating: assume=$ASSUME_COUNT external_body=$EXTERNAL_BODY_COUNT admit=$ADMIT_COUNT trusted=$TRUSTED_COUNT no_decreases=$NO_DECREASES_COUNT cfg_gate=$CFG_GATE_COUNT"
    CHEATING_DETAIL_FILE="${CRATE_SRC_DIR:-src}/cheating-detail.txt"
    get_cheating_detail "$CHEATING_DIR" > "$CHEATING_DETAIL_FILE"
    echo "  Detail: $CHEATING_DETAIL_FILE"
fi

echo ""

# ==============================================================================
# Function coverage analysis (tree-sitter AST via guardrails.py).
# ==============================================================================

echo "=== Function Coverage ==="

COVERAGE_PATHS="${CRATE_SRC_DIR:-.}"

# When --module is given, narrow coverage to that module's files only.
# Rust maps module foo::bar to both foo/bar.rs and foo/bar/ directory.
# MODULE=root means lib.rs only.
if [[ -n "$MODULE" && -n "$CRATE_SRC_DIR" ]]; then
    if [[ "$MODULE" == "root" ]]; then
        # Use the actual root file from cargo metadata, fallback to lib.rs/main.rs.
        if [[ -n "$CRATE_ROOT_FILE" && -f "$CRATE_ROOT_FILE" ]]; then
            COVERAGE_PATHS="$CRATE_ROOT_FILE"
        elif [[ -f "$CRATE_SRC_DIR/lib.rs" ]]; then
            COVERAGE_PATHS="$CRATE_SRC_DIR/lib.rs"
        elif [[ -f "$CRATE_SRC_DIR/main.rs" ]]; then
            COVERAGE_PATHS="$CRATE_SRC_DIR/main.rs"
        fi
    else
        MOD_PATH="${MODULE//:://}"  # foo::bar -> foo/bar
        COVERAGE_PATHS=""
        [[ -f "$CRATE_SRC_DIR/$MOD_PATH.rs" ]] && COVERAGE_PATHS="$CRATE_SRC_DIR/$MOD_PATH.rs"
        [[ -d "$CRATE_SRC_DIR/$MOD_PATH" ]]    && COVERAGE_PATHS="${COVERAGE_PATHS:+$COVERAGE_PATHS:}$CRATE_SRC_DIR/$MOD_PATH"
        [[ -z "$COVERAGE_PATHS" ]] && COVERAGE_PATHS="${CRATE_SRC_DIR:-.}"
    fi
fi

COVERAGE_JSON="$(VERUS_AI_DIR="$VERUS_AI_DIR" COVERAGE_PATHS="$COVERAGE_PATHS" python3 -c '
import sys, os, json
sys.path.insert(0, os.environ["VERUS_AI_DIR"])
from pathlib import Path
from guardrails import detect_coverage, CoverageReport

paths = os.environ["COVERAGE_PATHS"].split(":")
combined = CoverageReport()
for p in paths:
    r = detect_coverage(Path(p))
    combined.functions.extend(r.functions)
    combined.total_exec += r.total_exec
    combined.with_contracts += r.with_contracts
    combined.without_contracts += r.without_contracts
unverified = [f.name for f in combined.functions if not f.has_contracts]
print(json.dumps({
    "total": combined.total_exec,
    "verified": combined.with_contracts,
    "unverified": combined.without_contracts,
    "unverified_fns": unverified,
}))
' 2>/dev/null || echo '{"error":"unavailable"}')"

if echo "$COVERAGE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); sys.exit(0 if 'total' in d else 1)" 2>/dev/null; then
    COV_TOTAL="$(echo "$COVERAGE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['total'])")"
    COV_VERIFIED="$(echo "$COVERAGE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['verified'])")"
    COV_UNVERIFIED="$(echo "$COVERAGE_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['unverified'])")"

    if [[ "$COV_UNVERIFIED" -eq 0 ]]; then
        echo "  ✅ All $COV_TOTAL exec functions have contracts."
    elif [[ -n "$MODULE" ]]; then
        # Module-scoped: show full list inline.
        echo "  $COV_VERIFIED/$COV_TOTAL exec functions have contracts."
        echo "  Unverified functions:"
        echo "$COVERAGE_JSON" | python3 -c "import sys,json; [print(f'    - {n}') for n in json.load(sys.stdin)['unverified_fns']]"
    else
        # Whole-crate: too many to list inline — write to file.
        COVERAGE_FILE="${CRATE_SRC_DIR:-src}/coverage-unverified.txt"
        echo "$COVERAGE_JSON" | python3 -c "
import sys, json
d = json.load(sys.stdin)
with open('$COVERAGE_FILE', 'w') as f:
    f.write(f\"# Unverified exec functions: {d['unverified']}/{d['total']}\n\")
    for n in d['unverified_fns']:
        f.write(f'  {n}\n')
"
        echo "  $COV_VERIFIED/$COV_TOTAL exec functions have contracts."
        echo "  Unverified function list written to: $COVERAGE_FILE"
    fi
else
    echo "  ⚠️  Coverage analysis unavailable (tree-sitter required)."
    echo "     Set VERUS_AI_DIR to the verus-ai tool directory."
fi

echo ""
echo "=== Summary ==="
echo "  verification: $VERIFIED, $ERRORS (exit $VERUS_EXIT)"
echo "  cheating: assume=$ASSUME_COUNT external_body=$EXTERNAL_BODY_COUNT admit=$ADMIT_COUNT trusted=$TRUSTED_COUNT no_decreases=$NO_DECREASES_COUNT cfg_gate=$CFG_GATE_COUNT"
if [[ -n "$COV_TOTAL" ]]; then
    echo "  coverage: $COV_VERIFIED/$COV_TOTAL exec functions have contracts"
fi
if [[ $VERUS_EXIT -ne 0 ]]; then
    echo "  status: VERIFY_FAILED"
elif [[ "$CHEATING_FOUND" == "true" ]]; then
    echo "  status: CHEATING_DETECTED"
else
    echo "  status: CLEAN"
fi
echo ""

# ==============================================================================
# Log results.
# ==============================================================================

if [[ -n "$LOG_DIR" ]]; then
    VERUS_LOG_DIR="$LOG_DIR/verus-logs"
    mkdir -p "$VERUS_LOG_DIR"
    LOG_FILE="$VERUS_LOG_DIR/verus_${TIMESTAMP}.log"
    {
        echo "Timestamp: $TIMESTAMP"
        echo "Mode: $MODE"
        echo "Crate: ${CRATE_NAME:-N/A}"
        echo "Directory: ${VERUS_DIR:-N/A}"
        echo "Module: ${MODULE:-<all>}"
        echo "Exit code: $VERUS_EXIT"
        echo "$VERIFIED"
        echo "$ERRORS"
        echo ""
        echo "Cheating patterns:"
        echo "  assume=$ASSUME_COUNT"
        echo "  external_body=$EXTERNAL_BODY_COUNT"
        echo "  admit=$ADMIT_COUNT"
        echo "  trusted=$TRUSTED_COUNT"
        echo "  no_decreases=$NO_DECREASES_COUNT"
        echo "  cfg_gate=$CFG_GATE_COUNT"
        echo ""
        echo "--- Full output ---"
        cat "$TMPFILE"
    } > "$LOG_FILE"
    echo "Log written to: $LOG_FILE"
fi

# ==============================================================================
# Git commit (optional).
# ==============================================================================

if [[ "$GIT_COMMIT" == "true" ]]; then
    if [[ "$MODE" == "legacy" ]]; then
        cd - > /dev/null 2>&1
    fi

    # Only auto-commit on verus-* branches to avoid polluting other branches.
    CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
    if [[ "$CURRENT_BRANCH" != verus-* && "$CURRENT_BRANCH" != verus_* ]]; then
        echo "Skipping auto-commit: not on a verus-* branch (current: $CURRENT_BRANCH)"
    else

    # Stage verification-related files.
    if [[ -n "$CRATE_SRC_DIR" ]]; then
        git add -f "$CRATE_SRC_DIR"/*.rs "$CRATE_SRC_DIR"/*.spec.rs "$CRATE_SRC_DIR"/*.proof.rs 2>/dev/null || true
    fi
    if [[ -n "$LOG_DIR" ]]; then
        git add -f "$LOG_DIR" 2>/dev/null || true
    fi

    # Build commit message with all non-zero cheating counts.
    CHEAT_SUMMARY=""
    [[ $ADMIT_COUNT -gt 0 ]]          && CHEAT_SUMMARY+="admit=$ADMIT_COUNT "
    [[ $ASSUME_COUNT -gt 0 ]]         && CHEAT_SUMMARY+="assume=$ASSUME_COUNT "
    [[ $EXTERNAL_BODY_COUNT -gt 0 ]]  && CHEAT_SUMMARY+="external_body=$EXTERNAL_BODY_COUNT "
    [[ $TRUSTED_COUNT -gt 0 ]]        && CHEAT_SUMMARY+="trusted=$TRUSTED_COUNT "
    [[ $NO_DECREASES_COUNT -gt 0 ]]   && CHEAT_SUMMARY+="no_decreases=$NO_DECREASES_COUNT "
    [[ $CFG_GATE_COUNT -gt 0 ]]       && CHEAT_SUMMARY+="cfg_gate=$CFG_GATE_COUNT "
    CHEAT_SUMMARY="${CHEAT_SUMMARY% }"  # trim trailing space

    if [[ $VERUS_EXIT -eq 0 && "$CHEATING_FOUND" != "true" ]]; then
        COMMIT_MSG="[verus] verify PASS: ${LABEL} ($VERIFIED, $ERRORS)"
    elif [[ $VERUS_EXIT -eq 0 ]]; then
        COMMIT_MSG="[verus] verify PASS (cheating detected): ${LABEL} ($VERIFIED, $ERRORS, $CHEAT_SUMMARY)"
    else
        COMMIT_MSG="[verus] verify FAIL: ${LABEL} ($VERIFIED, $ERRORS)"
    fi

    if ! git diff --cached --quiet 2>/dev/null; then
        git commit -m "$COMMIT_MSG" --no-verify -q
    fi

    fi  # end verus-* branch check
fi

# Exit non-zero only if Verus verification itself failed.
# Cheating is reported as a warning but does not fail the build.
exit "$VERUS_EXIT"
