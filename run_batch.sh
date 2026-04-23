#!/usr/bin/env bash
# Batch verification pipeline for phys memory manager modules.
# Reads verification-order.txt, runs pipeline for each module top-down.
# kpool and upool start from proving (already have specs from prior work).
# All modules stop after cheating-elimination.

set -euo pipefail

VERUS_AI_DIR="$HOME/verus-ai-exp/verus-ai-phy-mm-0423"
NANVIX_DIR="$VERUS_AI_DIR/target-systems/nanvix"
ORDER_FILE="$NANVIX_DIR/verification-order.txt"

if [[ ! -f "$ORDER_FILE" ]]; then
    echo "ERROR: $ORDER_FILE not found"
    exit 1
fi

cd "$VERUS_AI_DIR"

total=$(wc -l < "$ORDER_FILE")
i=0

while IFS= read -r source_file; do
    [[ -z "$source_file" || "$source_file" == \#* ]] && continue
    i=$((i + 1))

    echo ""
    echo "========================================"
    echo "[$i/$total] $source_file"
    echo "========================================"

    from_flag=""
    # kpool and upool already have specs — start from proving
    if [[ "$source_file" == *"/kpool.rs" || "$source_file" == *"/upool.rs" ]]; then
        from_flag="--from proving"
    fi

    python3 run.py pipeline "$source_file" \
        --project-root "$NANVIX_DIR" \
        --to cheating-elimination \
        $from_flag \
        || echo "WARNING: pipeline failed for $source_file (exit $?), continuing..."

done < "$ORDER_FILE"

echo ""
echo "========================================"
echo "Batch complete: $i modules processed"
echo "========================================"
