#!/usr/bin/env bash
# Rating calibration: play StockShark against Stockfish at several fixed
# UCI_Elo levels and estimate StockShark's Elo from the score at each level.
#
# Usage:   ./gauntlet.sh [games_per_level] [elo_level ...]
# Example: ./gauntlet.sh 100 1500 1800 2100 2400
#
# StockShark_Elo(level) = level + EloDiff(fastchess), where EloDiff is derived
# from the score StockShark achieved against that level. The final estimate is
# the games-weighted average across levels, best trusted near the 50% crossover.
set -euo pipefail
source "$(dirname "$0")/config.sh"

GAMES="${1:-100}"; shift || true
LEVELS=("$@")
if [ ${#LEVELS[@]} -eq 0 ]; then LEVELS=(1500 1800 2100 2400); fi

if [ ! -x "$SS_BIN" ]; then echo "Build first: cargo build --release" >&2; exit 1; fi

STAMP="$(date +%Y%m%d_%H%M%S)"
SUMMARY="$RESULTS/gauntlet_${STAMP}.txt"
echo "StockShark gauntlet  TC=$TC  games/level=$GAMES  $(date)" | tee "$SUMMARY"
echo "level  score%   W-L-D    EloDiff        StockShark_Elo" | tee -a "$SUMMARY"

# rounds = games/2 because -repeat plays each opening twice (colors swapped)
ROUNDS=$(( (GAMES + 1) / 2 ))

for LVL in "${LEVELS[@]}"; do
  LOG="$RESULTS/gauntlet_${STAMP}_sf${LVL}.log"
  "$FASTCHESS" \
    -engine cmd="$SS_BIN" args="$SS_ARGS" name=StockShark \
    -engine cmd="$STOCKFISH" name="SF${LVL}" "option.UCI_LimitStrength=true" "option.UCI_Elo=${LVL}" \
    -each tc="$TC" timemargin="$TIMEMARGIN" \
    -rounds "$ROUNDS" -repeat -concurrency "$CONCURRENCY" \
    -openings file="$BOOK" format=epd order=random \
    -pgnout file="$RESULTS/games_${STAMP}_sf${LVL}.pgn" \
    > "$LOG" 2>&1 || true

  # fastchess prints: "Elo: <diff> +/- <err>" and a W/L/D "Games:" line
  DIFF=$(grep -Eo 'Elo: -?[0-9.]+ \+/- [0-9.a-z]+' "$LOG" | tail -1 | awk '{print $2}')
  ERR=$(grep -Eo 'Elo: -?[0-9.]+ \+/- [0-9.a-z]+'  "$LOG" | tail -1 | awk '{print $4}')
  STATS=$(grep -E 'Wins: .* Points:' "$LOG" | tail -1)
  W=$(echo "$STATS" | grep -Eo 'Wins: [0-9]+' | awk '{print $2}')
  L=$(echo "$STATS" | grep -Eo 'Losses: [0-9]+' | awk '{print $2}')
  D=$(echo "$STATS" | grep -Eo 'Draws: [0-9]+' | awk '{print $2}')
  PCT=$(echo "$STATS" | grep -Eo '\([0-9.]+ %\)' | tr -d '()% ')

  if [ -n "${DIFF:-}" ] && [ "$DIFF" != "-inf" ] && [ "$DIFF" != "inf" ]; then
    EST=$(awk -v l="$LVL" -v d="$DIFF" 'BEGIN{printf "%.0f", l+d}')
  else
    EST="(saturated)"
  fi
  printf "%-6s %-8s %s-%s-%s  %+8s +/- %-6s  %s\n" \
    "$LVL" "${PCT:-?}" "${W:-?}" "${L:-?}" "${D:-?}" "${DIFF:-?}" "${ERR:-?}" "$EST" | tee -a "$SUMMARY"
done

echo "" | tee -a "$SUMMARY"
echo "Logs + PGNs in $RESULTS (stamp $STAMP)" | tee -a "$SUMMARY"
