# StockShark

A chess engine written in Rust with a native graphical interface. Play against the engine, watch engine vs engine games, review any game move by move, and load Polyglot opening books.

![Game modes: Human vs Engine, Engine vs Human, Human vs Human, Engine vs Engine]

---

## Features

- **Four game modes** — Human vs Engine, Engine vs Human, Human vs Human, Engine vs Engine
- **Polyglot opening book** — drop any `.bin` book file (e.g. Titans.bin) into the same folder; the engine uses it automatically
- **Opening variety** — built-in book covering ~35 main opening lines; a new game seed is chosen each game so you see different openings each time
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
git clone https://github.com/yourname/StockShark
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

Search depth 1 → 2 → 3 → … until time runs out. Each shallower result informs move ordering for the next depth, generating more cutoffs.

#### Move ordering

1. Transposition table move (best move from a previous search of this position)
2. Captures — sorted by MVV-LVA (Most Valuable Victim / Least Valuable Attacker)
3. Killer moves — quiet moves that caused a beta cutoff in a sibling branch at this depth
4. History heuristic — quiet moves ranked by how often they've caused cutoffs

#### Quiescence search

At leaf nodes the engine extends the search with captures-only until the position is quiet. This prevents mis-evaluating positions mid-exchange.

#### Repetition detection

Every position hash on the path back through the game is tracked. If a position appears twice already, the move leading to it is scored as a draw. A contempt factor of −25 cp makes the engine prefer pressing for a win over accepting a repetition.

#### Transposition table

A 16 MB hash table stores results from previously searched positions. Entries record the depth searched, best move found, and whether the score is exact, a lower bound, or an upper bound. On a cache hit the engine can skip re-searching or improve move ordering.

---

### Evaluation (`src/engine/eval.rs`)

Score in centipawns, always from the perspective of the side to move. Positive = good for the mover.

#### Material

| Piece | Value |
|-------|-------|
| Pawn | 100 cp |
| Knight | 320 cp |
| Bishop | 330 cp |
| Rook | 500 cp |
| Queen | 900 cp |

#### Piece-square tables (PSTs)

Each piece has a 64-entry table of bonuses/penalties by square. The king has separate middlegame and endgame tables — in the middlegame it hides behind pawns (castling preferred); in the endgame it centralises to support mating attacks or avoid checkmate.

The engine detects **endgame phase** when total non-king material drops below 2000 cp (~one major piece and change), then switches king evaluation to the endgame table.

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
│   ├── eval.rs          — Material, PSTs, pawn structure, endgame bonuses
│   ├── search.rs        — Iterative deepening alpha-beta + quiescence
│   ├── tt.rs            — Transposition table
│   ├── book.rs          — Built-in hardcoded opening lines
│   └── polyglot.rs      — Polyglot .bin file reader + hash computation
├── uci/
│   └── mod.rs           — UCI protocol implementation
└── gui/
    └── app.rs           — egui board, eval bar, move list, engine thread
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
