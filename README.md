# StockShark

A chess engine written in Rust with a native graphical interface. Play against the engine, watch engine vs engine games, review any game move by move, and load Polyglot opening books.

![Game modes: Human vs Engine, Engine vs Human, Human vs Human, Engine vs Engine]

---

## Features

- **Four game modes** — Human vs Engine, Engine vs Human, Human vs Human, Engine vs Engine
- **Polyglot opening book** — drop any `.bin` book file (e.g. Titans.bin) into the same folder; the engine uses it automatically
- **Opening variety** — built-in book covering ~35 main opening lines; a new game seed is chosen each game so you see different openings each time
- **Modern search** — iterative-deepening alpha-beta with null-move pruning, PVS, late-move reductions, check extensions, aspiration windows, and quiescence with delta pruning
- **Tapered PeSTO evaluation** — phase-blended material/piece-square tables, pawn structure, mobility, and endgame king coordination
- **Measured strength** — **~2460 Elo** (Stockfish scale), improved ~+400 Elo through SPRT-validated testing (see [Strength & Testing](#strength--testing))
- **Adjustable search depth** — slider from 1 to 12 half-moves
- **Eval bar** — live centipawn score shown alongside the board
- **Game review** — browse every move with `←` / `→` keys or the `< >` buttons; click any move in the list to jump to it
- **Flip board** — toggle to view from Black's side
- **Draw detection** — 50-move rule, stalemate, insufficient material (K vs K, K+N vs K, K+B vs K, K+B vs K+B same colour)
- **Endgame knowledge** — evaluation guides the engine to checkmate in endings like K+R vs K, K+Q vs K
- **UCI support** — run as a UCI engine for Arena, Cutechess, Lichess bots, etc.

---

## Requirements

- **Rust** (stable) — install from [rustup.rs](https://rustup.rs). One command:

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

  On Windows, download and run the installer from [rustup.rs](https://rustup.rs).

- **A C linker** — comes with the OS on macOS and Linux. On Windows, install the [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (select "Desktop development with C++" workload).

No other dependencies are needed — everything else is fetched automatically by Cargo.

---

## Quick Start

```bash
# 1. Clone the repo
git clone https://github.com/TomH-2004/StockShark
cd StockShark

# 2. (Optional) Add an opening book — see "Opening Book" section below

# 3. Build and run
cargo run --release
```

The first build downloads dependencies and compiles everything (~60 seconds). Subsequent builds are much faster.

For a **UCI engine** (Arena, Cutechess, Lichess, etc.):

```bash
cargo run --release -- --uci
```

Or build a standalone binary and point your GUI at it:

```bash
cargo build --release
# Binary is at: target/release/stockshark  (target/release/stockshark.exe on Windows)
```

---

## Opening Book

StockShark supports the standard **Polyglot** opening book format (`.bin` files). These are derived from millions of grandmaster games and give the engine deep, theoretically sound opening knowledge far beyond the built-in lines.

### Getting a book

Several free Polyglot books are available online:

| Book | Size | Notes |
|------|------|-------|
| **Titans.bin** | ~2 MB | Large GM-quality book, widely recommended |
| Perfect2012.bin | ~10 MB | Another popular free option |
| komodo.bin | ~1 MB | Compact, strong |

Search for "Polyglot opening book bin download" — many sites host these for free.

### Using a book

1. Place the `.bin` file in the **same folder you run StockShark from** (the project root when using `cargo run`).
2. Name it `Titans.bin`, `titans.bin`, or `book.bin` — StockShark auto-loads those names at startup.
3. Or use the **Book → Load** text field in the toolbar to specify any path.

A green **📖 Loaded: Titans.bin** indicator appears in the toolbar when a book is active. The engine plays book moves instantly (shown as "Book..." in the status bar), then switches to its own search once the position leaves the book.

If no book file is present, StockShark falls back to its built-in opening lines, which cover ~35 main openings including Ruy Lopez, Italian, Sicilian, French, Caro-Kann, Queen's Gambit, King's Indian, Nimzo-Indian, and more.

---

## How to Play

### Game modes

Select a mode from the **Mode** dropdown in the toolbar:

| Mode | Description |
|------|-------------|
| Human vs Engine | You play White, engine plays Black |
| Engine vs Human | Engine plays White, you play Black |
| Human vs Human | Two humans take turns at the same board |
| Engine vs Engine | Watch the engine play itself |

### Making moves

Click a piece to select it — legal move destinations appear as green dots (empty squares) or green rings (captures). Click a destination to move. Click elsewhere to deselect.

### Controls

| Action | How |
|--------|-----|
| Select piece | Click it |
| Move | Click destination |
| Review previous move | `←` or `<` button |
| Review next move | `→` or `>` button |
| Jump to move | Click it in the move list |
| Return to live game | `>|` button or click `>` past the last move |
| Flip board | **Flip board** button |
| New game | **New Game** button |
| Adjust engine depth | **Depth** slider (1–12) |

While reviewing history the board dims, move hints disappear, and board clicks do nothing — navigate back to the live position to resume play.

---

## How It Works

### Board representation (`src/board/`)

The board is stored as **bitboards** — one 64-bit integer per piece type per colour (12 total). Bit N represents square N, so operations like "find all squares attacked by white pawns" become bitwise shifts and masks rather than loops.

**Position** holds:
- 12 piece bitboards + 2 combined occupancy bitboards
- Side to move, castling rights (4 bits), en passant target square, halfmove clock
- A `board: [Option<Piece>; 64]` array for O(1) piece lookup by square
- A **Zobrist hash** — a 64-bit value that uniquely identifies the position, built by XOR-ing random keys for each piece/square combination; updated incrementally on every make/unmake

---

### Move generation (`src/moves/`)

`generator.rs` produces all pseudo-legal moves; the search filters any that leave the king in check.

- **Pawns** — single/double pushes, all four promotions, diagonal captures, en passant
- **Sliding pieces** — ray-casting loops in all relevant directions, stopping at the first blocker
- **Castling** — enforces all five FIDE rules (king/rook unmoved, clear path, king not in/through check)

`make_move.rs` applies a move and returns an `Undo` struct with everything needed to reverse it (previous castling rights, ep square, halfmove clock, captured piece, Zobrist hash). `unmake_move` restores all of it without cloning the position.

---

### Search engine (`src/engine/search.rs`)

**Iterative deepening alpha-beta** with quiescence search.

#### Alpha-beta

Minimax with pruning: if we've already found a move guaranteeing +1.0, any branch where the opponent can force worse than that is skipped. In the best case this roughly halves the effective branching factor at every ply.

#### Iterative deepening

Search depth 1 → 2 → 3 → … until time runs out. Each shallower result informs move ordering for the next depth, generating more cutoffs. Under a time control the engine deepens as far as the clock allows (it allots roughly 1/25 of its remaining time plus most of the increment per move) and reserves a small safety margin so it always returns its move before the deadline — early versions capped out at a fixed depth and either wasted most of their time or, worse, forfeited on the clock.

#### Aspiration windows

From depth 5 onward the search opens a narrow window around the previous depth's score instead of a full `(-∞, +∞)` window. Most searches land inside it and finish faster; on a fail-high/fail-low the window is widened and the search retried. Near mate scores the window is opened fully to guarantee termination.

#### Move ordering

1. Transposition table move (best move from a previous search of this position)
2. Captures — sorted by MVV-LVA (Most Valuable Victim / Least Valuable Attacker)
3. Killer moves — quiet moves that caused a beta cutoff in a sibling branch at this depth
4. History heuristic — quiet moves ranked by how often they've caused cutoffs

#### Quiescence search

At leaf nodes the engine extends the search with captures-only until the position is quiet. This prevents mis-evaluating positions mid-exchange. **Delta pruning** skips captures that, even in the best case (winning the captured piece plus a margin), cannot raise the score to alpha.

#### Null-move pruning

If handing the opponent a free move still leaves the score above beta after a reduced-depth search, the position is almost certainly a cutoff and the subtree is pruned. Disabled when in check, in principal-variation nodes, near mate scores, and when the side to move has only pawns (where the zugzwang assumption breaks).

#### Principal variation search (PVS)

The first (best-ordered) move is searched with the full window; every later move is first probed with a zero-width window, and only re-searched with the full window if that probe unexpectedly beats alpha. Good move ordering makes the cheap probes succeed the vast majority of the time.

#### Late move reductions (LMR)

Quiet moves ordered late are searched at reduced depth first, with a reduction that grows logarithmically with depth and move count (`r ≈ ln(depth)·ln(moveNumber)/2.5`), eased for PV nodes and killer moves. If a reduced search beats alpha, it is re-searched at full depth.

#### Check extensions & forward pruning

- **Check extension** — when the side to move is in check, the search goes one ply deeper rather than dropping into quiescence, so forcing lines are resolved.
- **Reverse futility pruning** — if the static eval already beats beta by a wide, depth-scaled margin, cut immediately.
- **Futility pruning** — near the leaf, skip quiet moves that a margin-boosted static eval still can't lift to alpha.
- **Late move pruning** — at shallow depth, once enough quiet moves have been tried, skip the rest.

#### Repetition detection

Every position hash on the path back through the game is tracked. If a position appears twice already, the move leading to it is scored as a draw. A contempt factor of −25 cp makes the engine prefer pressing for a win over accepting a repetition.

#### Transposition table

A 16 MB hash table stores results from previously searched positions. Entries record the depth searched, best move found, and whether the score is exact, a lower bound, or an upper bound. On a cache hit the engine can skip re-searching or improve move ordering.

---

### Evaluation (`src/engine/eval.rs`)

Score in centipawns, always from the perspective of the side to move. Positive = good for the mover.

#### Tapered material + piece-square tables (PeSTO)

Material value and per-square placement are combined in **piece-square tables**, using the Texel-tuned [PeSTO](https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function) values. Each piece has **two** tables — a middlegame set and an endgame set — and the final score is a blend of the two, weighted by a **game phase** computed from the remaining material (knight/bishop = 1, rook = 2, queen = 4; full board = 24):

```
score = (mg_score · phase + eg_score · (24 − phase)) / 24
```

This tapering means values shift smoothly as pieces come off — a knight on the rim, a centralised king, an advanced pawn are all worth different amounts in the opening than in the endgame — instead of flipping abruptly at a fixed material threshold, which is how earlier versions worked.

#### Mobility

Knights, bishops, rooks, and queens receive a bonus for the number of squares they can reach (captures included), centred so a typically-placed piece scores near zero and weighted per piece type.

#### King safety

A **pawn shield** term rewards friendly pawns standing in front of the king (on its own file and the two neighbours) and penalises gaps, tapered so it fades toward the endgame where the king should be active. This deliberately stays cheap: an earlier version that also scanned the squares around the king for enemy attacks *lost* rating, because the per-node cost outweighed the extra knowledge (see the testing section).

#### Pawn structure

- **Doubled pawn penalty** (−15 cp per extra pawn on the same file)
- **Isolated pawn penalty** (−20 cp when no friendly pawn is on an adjacent file)
- **Passed pawn bonus** — grows with advancement: +15 cp at the starting rank, up to +115 cp on the 7th rank

#### Piece activity

- **Rook on open file** (+20 cp when no pawn of either colour blocks the file)
- **Rook on semi-open file** (+10 cp when only opponent pawns are on the file)
- **Bishop pair bonus** (+30 cp when a side has both bishops)

#### Endgame king coordination

When one side has a material advantage in an endgame, the evaluation adds:
- Up to **60 cp** for the attacking king being adjacent to the defending king
- Up to **60 cp** for the defending king being near a corner or edge

This guides the engine to execute K+R vs K, K+Q vs K, and other theoretical wins correctly instead of shuffling pieces until the 50-move rule.

#### Insufficient material detection

The game is immediately scored as a draw (and the GUI stops play) for:
- K vs K
- K+N vs K
- K+B vs K
- K+B vs K+B with both bishops on the same colour squares

---

### Opening book (`src/engine/book.rs`, `src/engine/polyglot.rs`)

Two layers:

1. **Polyglot file** (`polyglot.rs`) — reads any standard `.bin` book. Computes the Polyglot-compatible Zobrist hash (separate random table from the internal Zobrist, required for compatibility with downloaded books) and binary-searches the sorted file. Move selection is weighted by the book's stored frequency so common GM moves appear most often and rare sidelines appear occasionally.

2. **Internal hardcoded lines** (`book.rs`) — ~35 opening sequences covering all main systems. Used as a fallback when no Polyglot file is loaded or the position leaves the book. A per-game seed (from system time) picks among candidate moves so different games follow different lines.

---

### GUI (`src/gui/app.rs`)

Built with [egui](https://github.com/emilk/egui) / eframe.

- Board is rendered as 64 filled rectangles with Unicode piece glyphs (no image assets)
- Engine search runs on a background thread; the result is delivered via `Arc<Mutex<Option<SearchResult>>>` and polled each frame
- Book moves are queued through the same result channel so the board updates one move per frame, giving them the same visual rhythm as engine moves

---

### UCI protocol (`src/uci/`)

`cargo run --release -- --uci` activates a standard UCI loop compatible with Arena, Cutechess, and Lichess bots. Handles `uci`, `isready`, `ucinewgame`, `position`, `go`, and `stop`. Position history is tracked across `position` commands so repetition detection works correctly in tournament play.

---

## Strength & Testing

The engine's strength was measured empirically, and **every change was kept or discarded based on game results** rather than intuition. This section documents how that was done and what it showed.

### The test harness (`testing/`)

Games are run with [**fastchess**](https://github.com/Disservin/fastchess) as the match manager, with **Stockfish 18** as the reference opponent (throttled to fixed strengths via its `UCI_LimitStrength` / `UCI_Elo` options). Games are played at **8s + 0.08s** per side from a book of 99 balanced opening positions, 8 games in parallel. Two scripts drive everything:

| Script | Purpose |
|--------|---------|
| `testing/sprt.sh <new> <old>` | **A/B test** — does a new build beat the previous one? Runs an [SPRT](https://www.chessprogramming.org/Match_Statistics#SPRT) that stops as soon as the result is statistically decided. |
| `testing/gauntlet.sh <games> <elo…>` | **Rating calibration** — plays the engine against Stockfish at several fixed Elo levels and estimates the engine's rating from the score at each level. |

```bash
# Estimate the current rating against Stockfish
./testing/gauntlet.sh 120 2300 2400 2500

# Verify a change is an improvement before keeping it
cargo build --release && cp target/release/stockshark testing/bin/ss_new
./testing/sprt.sh testing/bin/ss_new testing/bin/ss_old
```

### Methodology

- **SPRT gate.** Each candidate build plays the previous one head-to-head. The SPRT tests H0 *"no improvement"* against H1 *"a real gain."* A change is kept only when the data accepts H1 (or, for smaller effects, when a large sample puts the whole confidence interval above zero). This catches regressions that look fine in a handful of games.
- **Absolute calibration.** Periodically the current build is played against Stockfish across a spread of `UCI_Elo` levels. Where the engine scores ~50%, its rating ≈ that Stockfish level. The score-to-Elo conversion is only trusted near 50%, because far from it the formula becomes very sensitive to noise.

### What the tests showed

Starting from a baseline calibrated at **~2060 Elo**, each change below was validated against the one before it:

| Build | Change | Measured result |
|-------|--------|-----------------|
| baseline | *(starting point)* | ~2060 Elo |
| Fix 1 | Reserve a time-safety margin (stop forfeiting on the clock) | correctness fix — see below |
| v2 | Search to the time limit instead of a fixed depth-6 cap | **+125 ± 35 Elo** |
| v3 | Null-move pruning, PVS, late-move reductions, check extensions | **+165 ± 40 Elo** |
| v4 | Aspiration windows + delta pruning | **+78 ± 23 Elo** → calibrated **~2350 Elo** |
| v5 | Reverse-futility / futility / late-move pruning, log-based LMR | **+15 ± 10 Elo** |
| v6 | **Tapered PeSTO evaluation** | **+165 ± 36 Elo** |
| v8 | Mobility + pawn-shield king safety | **+32 ± 15 Elo** → calibrated **~2460 Elo** |

Cumulatively that is roughly **+400 Elo of measured improvement**, from ~2060 to a calibrated **~2460** (57.9% vs Stockfish 2400, 44.6% vs 2500, 33.8% vs 2600 — three levels that agree on a ~2459 crossover).

Not every idea survived. A first attempt at full king safety — pawn shield *plus* counting enemy attacks on the squares around the king — measured **−16 Elo** and was rejected: evaluation runs at every leaf of the search, so the extra `is_attacked` calls slowed it more than the knowledge was worth. Cutting it back to the cheap pawn-shield-only term turned the same batch into a **+32** gain. This is exactly the kind of result the SPRT gate exists to catch.

### How results drove the changes

The testing loop repeatedly surfaced things that would have been invisible otherwise:

- **The engine was losing every game on time.** The very first match showed a clean sweep of losses — not from bad moves, but from overrunning its move deadline by a few milliseconds, which strict GUIs forfeit. That made the time-safety margin the first fix, before any strength work.
- **It was throttling itself.** The engine hard-capped its search at depth 6 and left most of its clock unused. Removing that cap (v2) was the single cheapest large gain.
- **Evaluation, not search, was the ceiling.** Once the search had the standard pruning/reduction machinery, further search tweaks (v5) returned little. Rewriting the evaluation to the tapered PeSTO tables (v6) was worth as much as the entire earlier search-heuristics batch — which is why it was prioritised.
- **A bug found during sanity checks.** Verifying v5 revealed the aspiration-window loop could spin forever on a position where the engine is being mated (and in games, burn its whole clock). That was fixed as part of the same batch.
- **A rejected feature.** Full king safety (pawn shield + a scan of enemy attacks around the king) made the engine *weaker* because of its per-node cost; only the cheap pawn-shield half survived testing. Cost matters as much as correctness when a term runs at every leaf.

> **Caveat on the numbers.** Ratings here are on Stockfish's `UCI_Elo` scale at a fast time control, which tends to read higher than public rating lists (CCRL/FIDE). Treat them as a consistent *relative* yardstick for measuring progress, not a guaranteed CCRL figure.

---

## Project Structure

```
src/
├── main.rs              — entry point, GUI vs UCI mode switch
├── board/
│   ├── bitboard.rs      — Bitboard type with iterator and shift helpers
│   ├── position.rs      — Position, Move encoding, FEN parse/generate
│   └── zobrist.rs       — Zobrist key generation (internal LCG-based)
├── moves/
│   ├── generator.rs     — Pseudo-legal move generation for all piece types
│   └── make_move.rs     — make_move / unmake_move with full undo support
├── engine/
│   ├── eval.rs          — Tapered PeSTO eval, pawn structure, mobility, endgame bonuses
│   ├── search.rs        — Iterative deepening alpha-beta + quiescence + pruning/reductions
│   ├── tt.rs            — Transposition table
│   ├── book.rs          — Built-in hardcoded opening lines
│   └── polyglot.rs      — Polyglot .bin file reader + hash computation
├── uci/
│   └── mod.rs           — UCI protocol implementation
└── gui/
    └── app.rs           — egui board, eval bar, move list, engine thread

testing/                 — strength-testing harness (see "Strength & Testing")
├── sprt.sh              — A/B test a new build against an old one (SPRT)
├── gauntlet.sh          — Estimate rating vs Stockfish at fixed Elo levels
├── config.sh            — Shared paths / match settings
└── books/               — Opening positions for varied games
```

---

## Building a Release Binary

```bash
cargo build --release
```

The binary is written to `target/release/stockshark` (or `stockshark.exe` on Windows). Copy it anywhere — it has no runtime dependencies.

To distribute to someone else: send them the binary and (optionally) a `.bin` opening book file placed in the same directory. No Rust installation required on the recipient's machine.

> **macOS note:** macOS may block the binary with "unidentified developer". Right-click the binary in Finder and choose **Open**, or run `xattr -d com.apple.quarantine stockshark` in the terminal.

> **Windows note:** Windows Defender may flag an unsigned binary. Click **More info → Run anyway** if you trust the source, or sign the binary with a code signing certificate to avoid the warning.
