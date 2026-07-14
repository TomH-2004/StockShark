#!/usr/bin/env bash
# SPRT A/B test: does a new StockShark build beat the old one?
# Use this to validate every strength change before keeping it.
#
# Usage:  ./sprt.sh <new_binary> <old_binary> [name_new] [name_old]
# Example: cargo build --release
#          cp target/release/stockshark testing/bin/ss_new
#          ./sprt.sh testing/bin/ss_new testing/bin/ss_base new base
#
# SPRT bounds: H0 = elo0 (no gain), H1 = elo1 (real gain). It stops early once
# the result is statistically decided. "H1 accepted" = the change is an
# improvement; "H0 accepted" = it is not.
set -euo pipefail
source "$(dirname "$0")/config.sh"

NEW="${1:?need new binary}"
OLD="${2:?need old binary}"
NAME_NEW="${3:-new}"
NAME_OLD="${4:-base}"

ELO0="${ELO0:-0}"
ELO1="${ELO1:-15}"
MAXGAMES="${MAXGAMES:-2000}"
ROUNDS=$(( MAXGAMES / 2 ))

STAMP="$(date +%Y%m%d_%H%M%S)"
LOG="$RESULTS/sprt_${NAME_NEW}_vs_${NAME_OLD}_${STAMP}.log"

echo "SPRT $NAME_NEW vs $NAME_OLD  TC=$TC  elo0=$ELO0 elo1=$ELO1  maxgames=$MAXGAMES"
"$FASTCHESS" \
  -engine cmd="$NEW" args="$SS_ARGS" name="$NAME_NEW" \
  -engine cmd="$OLD" args="$SS_ARGS" name="$NAME_OLD" \
  -each tc="$TC" timemargin="$TIMEMARGIN" \
  -rounds "$ROUNDS" -repeat -concurrency "$CONCURRENCY" \
  -openings file="$BOOK" format=epd order=random \
  -sprt elo0="$ELO0" elo1="$ELO1" alpha=0.05 beta=0.05 \
  2>&1 | tee "$LOG" | grep -E 'Elo:|SPRT|LLR|Games:|accepted|Ptnml'

echo "Full log: $LOG"
