# StockShark

A chess engine written in Rust with a native graphical interface. Play against the engine, watch engine vs engine games, or review any game move by move.

---

## Running

```bash
# Launch the GUI
cargo run --release

# Run as a UCI-compatible engine (for use with Arena, Lichess bots, etc.)
cargo run --release -- --uci
```

---

## How It Works

StockShark is split into four layers that build on each other: board representation, move generation, search, and the GUI.

### Board Representation (`src/board/`)

The board is stored as a set of **bitboards** — one 64-bit integer per piece type per colour (12 total). Each bit represents one square, so operations like "find all squares attacked by white pawns" become a handful of bitwise shifts and masks rather than loops over every piece. This is the standard approach in fast chess engines.

**Position** (`position.rs`) holds:
- The 12 piece bitboards + 2 combined occupancy bitboards (white/black)
- Side to move, castling rights (4 bits), en passant target square
- A `board: [Option<Piece>; 64]` array for O(1) piece lookup by square
- A **Zobrist hash** — a single 64-bit number that uniquely identifies the position, built by XOR-ing random keys for each piece/square combination

**Zobrist hashing** (`zobrist.rs`) generates all random keys at startup using a simple LCG. The hash is updated incrementally as moves are made and unmade, so it stays in sync without recomputing from scratch each time. It's used for repetition detection.

---

### Move Generation (`src/moves/`)

`generator.rs` produces all pseudo-legal moves for a position, then the search filters out any that leave the king in check.

**Pawn moves** handle single pushes, double pushes from the starting rank, all four promotion pieces, diagonal captures, and en passant captures.

**Sliding pieces** (bishops, rooks, queens) use a simple ray-casting loop: walk in each direction, add squares, stop at the first blocker (include it if it's an enemy, exclude it if friendly).

**Castling** enforces all five rules:
1. Neither the king nor the relevant rook has moved (tracked via castling rights bits cleared on first move from those squares)
2. All squares between them are empty
3. The king is not currently in check
4. The king does not pass through an attacked square
5. The king does not land on an attacked square

`make_move.rs` applies a move to a `Position` and returns an `Undo` struct containing everything needed to reverse it — the previous castling rights, en passant square, halfmove clock, Zobrist hash, and any captured piece. `unmake_move` restores all of this exactly, so the search can explore moves without cloning the full position.

---

### Search Engine (`src/engine/`)

The engine finds the best move using **iterative deepening alpha-beta search**.

#### Alpha-Beta Pruning

At its core the search is minimax: try every move, assume the opponent plays optimally, pick the move that leads to the best outcome. Alpha-beta pruning makes this practical — if we've already found a move that guarantees a score of +1.0, we can stop searching any branch where the opponent can force a result below that. In the best case this cuts the search tree roughly in half at each ply, allowing roughly twice the depth in the same time.

#### Iterative Deepening

Rather than searching directly to depth N, the engine searches depth 1, then depth 2, then depth 3, and so on until time runs out. This seems wasteful but is actually faster, because each completed shallower search informs move ordering for the next — finding good moves early causes far more alpha-beta cutoffs.

#### Move Ordering

The order moves are searched matters enormously for alpha-beta efficiency. StockShark orders moves as:

1. **Captures** — sorted by MVV-LVA (Most Valuable Victim / Least Valuable Attacker): prefer capturing a queen with a pawn over capturing a pawn with a queen
2. **Killer moves** — quiet moves that caused a beta cutoff at this depth in a sibling branch
3. **History heuristic** — quiet moves scored by how often they've caused cutoffs across the whole search

#### Quiescence Search

At the leaves of the main search, instead of calling the evaluator immediately, a quiescence search continues searching captures-only until the position is "quiet" (no captures available). This prevents the engine from stopping mid-sequence — for example, evaluating a position where it just captured a queen but hasn't seen the recapture yet.

#### Repetition Detection & Contempt

The search tracks the Zobrist hash of every position on the current path back through the game. If a position appears twice already (making this the third occurrence), the move that leads there is scored as a draw. The draw score includes a **contempt factor** of −25 centipawns: the engine treats a draw as slightly bad from its own perspective, so a winning engine will keep pressing for a win rather than repeat.

---

### Evaluation (`src/engine/eval.rs`)

The evaluator assigns a score to any position in centipawns (100 cp = one pawn). It runs in two parts:

**Material**: each piece has a fixed value — pawns 100, knights 320, bishops 330, rooks 500, queens 900.

**Piece-square tables (PSTs)**: each piece type has a 64-entry table of bonuses/penalties depending on which square it occupies. Examples of what the tables encode:
- Pawns get a strong incentive (−20 cp penalty) for sitting on their starting squares, rewarding central advances like e4/d4
- Knights are penalised on edge squares (−50 cp in corners) and rewarded in the centre (+20 cp on d4/e4/d5/e5)
- The king is rewarded for castling (+30 cp on g1/c1) and heavily penalised for sitting in the centre during the middlegame
- Rooks score a bonus on the seventh rank

The score is always returned relative to the side to move, so a positive number always means "good for whoever's turn it is."

---

### GUI (`src/gui/`)

The interface is built with [egui](https://github.com/emilk/egui) via eframe.

**Board rendering** draws 64 squares with colour-coded highlights:
- Yellow — the move that was just played
- Green — the selected piece and its legal move destinations
- Blue — move highlights while reviewing game history

**Legal move hints** appear as dots on reachable empty squares and rings around capturable pieces (including en passant targets, which are empty squares but still shown as captures).

**Engine search** runs on a background thread so the UI never freezes. The result is delivered via an `Arc<Mutex<Option<SearchResult>>>` and polled each frame.

**Game review** — after any number of moves, you can browse the full game history:
- `|<` `<` `>` `>|` buttons in the move list panel
- `←` / `→` arrow keys anywhere in the window
- Click any move in the move list to jump directly to that position

While reviewing, the board dims slightly and a banner shows which move you're viewing. Move hints are hidden and clicks on the board do nothing — you have to return to the live position to play.

---

### UCI Protocol (`src/uci/`)

Running with `--uci` activates a standard UCI (Universal Chess Interface) loop. This lets StockShark work with external chess GUIs (Arena, Cutechess, etc.) or play on Lichess as a bot. The engine handles `position`, `go`, `stop`, `ucinewgame`, and reports `bestmove` after each search. Position history is tracked across `position` commands so repetition detection works correctly in tournament play.

---

## Project Structure

```
src/
├── main.rs              — entry point, GUI vs UCI mode switch
├── board/
│   ├── bitboard.rs      — Bitboard type with iterator and shift operations
│   ├── position.rs      — Position, Move encoding, FEN parsing/generation
│   └── zobrist.rs       — Zobrist hash key generation
├── moves/
│   ├── generator.rs     — Pseudo-legal move generation for all piece types
│   └── make_move.rs     — make_move / unmake_move with full undo support
├── engine/
│   ├── eval.rs          — Material + piece-square table evaluation
│   └── search.rs        — Iterative deepening alpha-beta with quiescence
├── uci/
│   └── mod.rs           — UCI protocol implementation
└── gui/
    └── app.rs           — egui board, move list, navigation, engine thread
```
