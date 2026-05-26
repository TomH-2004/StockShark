use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::board::position::{Move, Position};
use crate::engine::eval::{evaluate, DRAW_SCORE, MATE_SCORE};
use crate::moves::generator::MoveGen;
use crate::moves::make_move::{make_move, unmake_move};

/// Penalty (in centipawns, from the side-to-move's perspective) for accepting a draw
/// by repetition. A winning engine treats draws as slightly bad; a losing engine would
/// prefer a draw, but that's handled by the normal search returning negative scores anyway.
const CONTEMPT: i32 = -25;

pub struct SearchResult {
    pub best_move: Move,
    pub score: i32,
    pub depth: u32,
    pub nodes: u64,
    pub pv: Vec<Move>,
}

pub struct Engine {
    pub stop: Arc<AtomicBool>,
    nodes: u64,
    /// Killer moves: [depth][slot]
    killers: [[Move; 2]; 64],
    /// History heuristic: [from][to]
    history: [[i32; 64]; 64],
    /// Zobrist hashes of all positions played in the actual game before this search.
    /// The search uses this to detect repetitions against the real game history.
    pub game_history: Vec<u64>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            nodes: 0,
            killers: [[Move::default(); 2]; 64],
            history: [[0; 64]; 64],
            game_history: Vec::new(),
        }
    }

    pub fn search(&mut self, pos: &mut Position, max_depth: u32, time_ms: Option<u64>) -> SearchResult {
        self.stop.store(false, Ordering::Relaxed);
        self.nodes = 0;
        self.killers = [[Move::default(); 2]; 64];
        self.history = [[0; 64]; 64];

        let deadline = time_ms.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));

        let mut best = Move::default();
        let mut best_score = -MATE_SCORE;
        let mut pv = Vec::new();

        // search_stack holds hashes of positions on the current search path.
        // It starts populated with the real game history so repetitions against
        // already-played positions are detected correctly.
        let mut search_stack = self.game_history.clone();

        for depth in 1..=max_depth {
            if self.stop.load(Ordering::Relaxed) { break; }
            if let Some(dl) = deadline {
                if Instant::now() >= dl { break; }
            }

            let mut current_pv = Vec::new();
            let score = self.negamax(pos, depth, 0, -MATE_SCORE, MATE_SCORE, &mut current_pv, deadline, &mut search_stack);

            if !self.stop.load(Ordering::Relaxed) || depth == 1 {
                best_score = score;
                if !current_pv.is_empty() {
                    best = current_pv[0];
                    pv = current_pv;
                }
            }
        }

        SearchResult {
            best_move: best,
            score: best_score,
            depth: max_depth,
            nodes: self.nodes,
            pv,
        }
    }

    fn negamax(
        &mut self,
        pos: &mut Position,
        depth: u32,
        ply: usize,
        mut alpha: i32,
        beta: i32,
        pv: &mut Vec<Move>,
        deadline: Option<Instant>,
        search_stack: &mut Vec<u64>,
    ) -> i32 {
        // Repetition detection: count how many times this position has occurred on the
        // current path (including real game history loaded into search_stack). Two prior
        // occurrences means this move would be the third — a draw by repetition.
        let current_hash = pos.hash;
        let repetitions = search_stack.iter().filter(|&&h| h == current_hash).count();
        if repetitions >= 2 {
            return CONTEMPT;
        }

        if depth == 0 {
            return self.quiescence(pos, alpha, beta);
        }

        if self.nodes & 0xFFF == 0 {
            if self.stop.load(Ordering::Relaxed) { return 0; }
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    self.stop.store(true, Ordering::Relaxed);
                    return 0;
                }
            }
        }

        self.nodes += 1;

        let mut moves = MoveGen::generate(pos);
        self.order_moves(pos, &mut moves, ply);

        let mut legal = 0;
        let mut best_pv = Vec::new();

        for mv in moves {
            let undo = make_move(pos, mv);

            // Skip illegal moves (king left in check)
            let them = 1 - pos.side as usize;
            if MoveGen::is_attacked(pos, pos.king_sq[them], pos.side as usize) {
                unmake_move(pos, mv, undo);
                continue;
            }

            legal += 1;

            // Push the pre-move hash so the child can detect repetition against this position
            search_stack.push(current_hash);
            let mut child_pv = Vec::new();
            let score = -self.negamax(pos, depth - 1, ply + 1, -beta, -alpha, &mut child_pv, deadline, search_stack);
            search_stack.pop();

            unmake_move(pos, mv, undo);

            if self.stop.load(Ordering::Relaxed) { return 0; }

            if score >= beta {
                if !mv.is_capture() && ply < 64 {
                    if self.killers[ply][0] != mv {
                        self.killers[ply][1] = self.killers[ply][0];
                        self.killers[ply][0] = mv;
                    }
                }
                self.history[mv.from_sq() as usize][mv.to_sq() as usize] += (depth * depth) as i32;
                return beta;
            }

            if score > alpha {
                alpha = score;
                best_pv = child_pv;
                best_pv.insert(0, mv);
            }
        }

        if legal == 0 {
            if MoveGen::in_check(pos) {
                return -(MATE_SCORE - ply as i32);
            } else {
                return DRAW_SCORE;
            }
        }

        *pv = best_pv;
        alpha
    }

    fn quiescence(&mut self, pos: &mut Position, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;

        let stand_pat = evaluate(pos);
        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }

        let mut captures = MoveGen::generate_captures(pos);
        captures.sort_by_key(|m| -mvv_lva(pos, *m));

        for mv in captures {
            let undo = make_move(pos, mv);
            let them = 1 - pos.side as usize;
            if MoveGen::is_attacked(pos, pos.king_sq[them], pos.side as usize) {
                unmake_move(pos, mv, undo);
                continue;
            }
            let score = -self.quiescence(pos, -beta, -alpha);
            unmake_move(pos, mv, undo);

            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }

        alpha
    }

    fn order_moves(&self, pos: &Position, moves: &mut Vec<Move>, ply: usize) {
        moves.sort_by_key(|mv| {
            let mut score = 0i32;
            if mv.is_capture() {
                score += 10_000 + mvv_lva(pos, *mv);
            } else if ply < 64 && (self.killers[ply][0] == *mv || self.killers[ply][1] == *mv) {
                score += 9_000;
            } else {
                score += self.history[mv.from_sq() as usize][mv.to_sq() as usize];
            }
            -score
        });
    }
}

fn mvv_lva(pos: &Position, mv: Move) -> i32 {
    let attacker = pos.board[mv.from_sq() as usize].map(|p| p.kind.material_value()).unwrap_or(0);
    let victim   = pos.board[mv.to_sq() as usize].map(|p| p.kind.material_value()).unwrap_or(0);
    victim * 10 - attacker
}
