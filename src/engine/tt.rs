use crate::board::position::Move;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtFlag {
    Exact = 0,
    Lower = 1, // score is a lower bound (beta cutoff node)
    Upper = 2, // score is an upper bound (all-node, failed low)
}

#[derive(Clone, Copy)]
struct TtEntry {
    key: u32,        // upper 32 bits of Zobrist hash, for collision detection
    score: i32,
    best_move: Move,
    depth: u8,
    flag: TtFlag,
}

const EMPTY: TtEntry = TtEntry {
    key: 0,
    score: 0,
    best_move: Move(0),
    depth: 0,
    flag: TtFlag::Exact,
};

pub struct TranspositionTable {
    entries: Vec<TtEntry>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let count = ((size_mb * 1024 * 1024) / std::mem::size_of::<TtEntry>())
            .next_power_of_two()
            >> 1;
        let count = count.max(1);
        Self { entries: vec![EMPTY; count], mask: count - 1 }
    }

    pub fn clear(&mut self) {
        self.entries.fill(EMPTY);
    }

    /// Returns `(cutoff_score, tt_move)`.
    /// `cutoff_score` is `Some` when the stored result is deep enough and
    /// the flag allows a cutoff at the current (alpha, beta) window.
    /// `tt_move` is the best move recorded (may be null).
    pub fn probe(&self, hash: u64, depth: u32, alpha: i32, beta: i32, ply: usize) -> (Option<i32>, Move) {
        let e = self.entries[hash as usize & self.mask];
        if e.key != (hash >> 32) as u32 {
            return (None, Move::default());
        }

        let tt_move = e.best_move;

        if (e.depth as u32) < depth {
            return (None, tt_move);
        }

        let score = score_from_tt(e.score, ply);
        let cutoff = match e.flag {
            TtFlag::Exact => true,
            TtFlag::Lower => score >= beta,
            TtFlag::Upper => score <= alpha,
        };

        (if cutoff { Some(score) } else { None }, tt_move)
    }

    pub fn store(&mut self, hash: u64, depth: u32, score: i32, flag: TtFlag, best_move: Move, ply: usize) {
        let idx = hash as usize & self.mask;
        let existing = &self.entries[idx];
        let key = (hash >> 32) as u32;

        // Depth-preferred replacement: keep deeper entries from the same position,
        // but always overwrite stale entries (different key).
        if existing.key == key && (existing.depth as u32) > depth {
            return;
        }

        self.entries[idx] = TtEntry {
            key,
            score: score_to_tt(score, ply),
            best_move,
            depth: depth.min(255) as u8,
            flag,
        };
    }
}

// Mate scores are stored relative to the current node so they remain correct
// when retrieved at a different ply.
fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score > 90_000 { score + ply as i32 } else if score < -90_000 { score - ply as i32 } else { score }
}

fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score > 90_000 { score - ply as i32 } else if score < -90_000 { score + ply as i32 } else { score }
}
