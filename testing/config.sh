# Shared config for the StockShark test harness.
# Sourced by gauntlet.sh and sprt.sh.

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTING="$REPO/testing"

FASTCHESS="$TESTING/bin/fastchess"
STOCKFISH="$(command -v stockfish)"
BOOK="$TESTING/books/openings.epd"
RESULTS="$TESTING/results"

# StockShark UCI binary (built with: cargo build --release)
SS_BIN="$REPO/target/release/stockshark"
SS_ARGS="--uci"

# Default match settings (override via env before calling the scripts)
TC="${TC:-8+0.08}"          # time control: base_seconds+increment
TIMEMARGIN="${TIMEMARGIN:-40}"   # ms tolerance for time overruns
CONCURRENCY="${CONCURRENCY:-8}"  # parallel games (machine has 10 cores)

mkdir -p "$RESULTS"
