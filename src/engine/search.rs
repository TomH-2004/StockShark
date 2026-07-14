use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::board::position::{Color, Move, Position};
use crate::engine::eval::{evaluate, DRAW_SCORE, MATE_SCORE};
use crate::engine::tt::{TranspositionTable, TtFlag};
use crate::moves::generator::MoveGen;
use crate::moves::make_move::{make_move, make_null_move, unmake_move, unmake_null_move};

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
    killers: [[Move; 2]; 64],
    history: [[i32; 64]; 64],
    pub game_history: Vec<u64>,
    pub tt: TranspositionTable,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            nodes: 0,
            killers: [[Move::default(); 2]; 64],
            history: [[0; 64]; 64],
            game_history: Vec::new(),
            tt: TranspositionTable::new(16),
        }
    }

    pub fn search(&mut self, pos: &mut Position, max_depth: u32, time_ms: Option<u64>) -> SearchResult {
        self.stop.store(false, Ordering::Relaxed);
        self.nodes = 0;
        self.killers = [[Move::default(); 2]; 64];
        self.history = [[0; 64]; 64];

        // Reserve a safety margin so the bestmove is always emitted before the
        // real deadline. Strict GUIs forfeit on even a 1ms overrun, so we target
        // slightly less than the allotted time (5% of budget, clamped to 10-60ms).
        let deadline = time_ms.map(|ms| {
            let safety = (ms / 20).clamp(10, 60).min(ms);
            Instant::now() + std::time::Duration::from_millis(ms - safety)
        });

        let mut best = Move::default();
        let mut best_score = -MATE_SCORE;
        let mut pv = Vec::new();

        let mut search_stack = self.game_history.clone();

        for depth in 1..=max_depth {
            if self.stop.load(Ordering::Relaxed) { break; }
            if let Some(dl) = deadline {
                if Instant::now() >= dl { break; }
            }

            let mut current_pv = Vec::new();
            let score;
            if depth <= 4 {
                // Shallow depths are cheap; search the full window for a solid seed.
                score = self.negamax(pos, depth, 0, -MATE_SCORE, MATE_SCORE, &mut current_pv, deadline, &mut search_stack, true);
            } else {
                // Aspiration window: assume the score is near the previous depth's
                // and search a narrow window, widening on a fail-high/fail-low.
                let mut delta = 30;
                let mut alpha = (best_score - delta).max(-MATE_SCORE);
                let mut beta = (best_score + delta).min(MATE_SCORE);
                loop {
                    current_pv.clear();
                    let s = self.negamax(pos, depth, 0, alpha, beta, &mut current_pv, deadline, &mut search_stack, true);
                    if self.stop.load(Ordering::Relaxed) { score = s; break; }
                    if s <= alpha {
                        // Fail low: relax alpha downward, nudge beta toward it for stability.
                        beta = (alpha + beta) / 2;
                        alpha = (s - delta).max(-MATE_SCORE);
                    } else if s >= beta {
                        // Fail high: relax beta upward.
                        beta = (s + delta).min(MATE_SCORE);
                    } else {
                        score = s;
                        break;
                    }
                    delta *= 2;
                    if delta > 2000 { alpha = -MATE_SCORE; beta = MATE_SCORE; }
                }
            }

            if !self.stop.load(Ordering::Relaxed) || depth == 1 {
                best_score = score;
                if !current_pv.is_empty() {
                    best = current_pv[0];
                    pv = current_pv;
                }
            }
        }

        // Fallback: if iterative deepening never produced a move (e.g. stopped before
        // the first node was fully searched), return the first legal move so the game
        // never freezes on a null result.
        if best.is_null() {
            for mv in MoveGen::generate(pos) {
                let undo = make_move(pos, mv);
                let them = 1 - pos.side as usize;
                let legal = !MoveGen::is_attacked(pos, pos.king_sq[them], pos.side as usize);
                unmake_move(pos, mv, undo);
                if legal {
                    best = mv;
                    break;
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

    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        pos: &mut Position,
        mut depth: u32,
        ply: usize,
        mut alpha: i32,
        beta: i32,
        pv: &mut Vec<Move>,
        deadline: Option<Instant>,
        search_stack: &mut Vec<u64>,
        can_null: bool,
    ) -> i32 {
        let current_hash = pos.hash;
        // A PV node has a full (non-null) window; child zero-window searches don't.
        let is_pv = beta - alpha > 1;

        // Draw detection: repetition — skip at ply=0 so the engine always
        // returns a legal move from the root even in repeated positions.
        if ply > 0 {
            let repetitions = search_stack.iter().filter(|&&h| h == current_hash).count();
            if repetitions >= 2 {
                return CONTEMPT;
            }
        }

        // Draw detection: fifty-move rule
        if pos.halfmove_clock >= 100 {
            return DRAW_SCORE;
        }

        // Check extension: when in check, search one ply deeper and never fall
        // into quiescence (which can't resolve checks).
        let in_check = MoveGen::in_check(pos);
        if in_check && ply > 0 {
            depth += 1;
        }

        if depth == 0 {
            return self.quiescence(pos, alpha, beta);
        }

        // Transposition table probe
        let (tt_score, tt_move) = self.tt.probe(current_hash, depth, alpha, beta, ply);
        if let Some(score) = tt_score {
            // Don't cut off at the root — we need a best_move to return.
            if ply > 0 {
                return score;
            }
        }

        // Time check every 4096 nodes
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

        // Null-move pruning: hand the opponent a free move; if we're still above
        // beta after a shallow search, this node is almost certainly a cutoff.
        // Skipped in check, in PV nodes, near mate scores, and when the side to
        // move has only pawns (where zugzwang makes passing unsafe).
        if !is_pv
            && can_null
            && !in_check
            && depth >= 3
            && ply > 0
            && beta.abs() < MATE_SCORE - 1000
            && has_non_pawn_material(pos, pos.side)
        {
            let undo = make_null_move(pos);
            let r = 2 + depth / 4;
            let reduced = depth.saturating_sub(1 + r);
            search_stack.push(current_hash);
            let mut null_pv = Vec::new();
            let score = -self.negamax(pos, reduced, ply + 1, -beta, -beta + 1, &mut null_pv, deadline, search_stack, false);
            search_stack.pop();
            unmake_null_move(pos, undo);

            if self.stop.load(Ordering::Relaxed) { return 0; }
            if score >= beta {
                return beta;
            }
        }

        let orig_alpha = alpha;
        let mut moves = MoveGen::generate(pos);
        self.order_moves(pos, &mut moves, ply, tt_move);

        let mut legal = 0;
        let mut best_pv = Vec::new();
        let mut best_move = Move::default();

        for mv in moves {
            let undo = make_move(pos, mv);

            let them = 1 - pos.side as usize;
            if MoveGen::is_attacked(pos, pos.king_sq[them], pos.side as usize) {
                unmake_move(pos, mv, undo);
                continue;
            }

            legal += 1;

            search_stack.push(current_hash);
            let mut child_pv = Vec::new();
            let new_depth = depth - 1;

            let score = if legal == 1 {
                // First (expected-best) move: search with the full window.
                -self.negamax(pos, new_depth, ply + 1, -beta, -alpha, &mut child_pv, deadline, search_stack, true)
            } else {
                // Late move reduction: quiet moves ordered late are searched
                // shallower first, then re-searched at full depth only if they
                // look like they might beat alpha.
                let mut reduction = 0u32;
                if depth >= 3 && legal > 3 && !mv.is_capture() && !mv.is_promotion() && !in_check {
                    reduction = 1;
                    if legal > 6 { reduction += 1; }
                    if !is_pv { reduction += 1; }
                    reduction = reduction.min(new_depth);
                }

                // Principal variation search: zero-window probe first.
                let mut s = -self.negamax(pos, new_depth - reduction, ply + 1, -alpha - 1, -alpha, &mut child_pv, deadline, search_stack, true);
                // Reduced search beat alpha → re-search at full depth.
                if reduction > 0 && s > alpha {
                    child_pv.clear();
                    s = -self.negamax(pos, new_depth, ply + 1, -alpha - 1, -alpha, &mut child_pv, deadline, search_stack, true);
                }
                // Score landed inside the window → re-search with the full window.
                if s > alpha && s < beta {
                    child_pv.clear();
                    s = -self.negamax(pos, new_depth, ply + 1, -beta, -alpha, &mut child_pv, deadline, search_stack, true);
                }
                s
            };

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
                self.tt.store(current_hash, depth, beta, TtFlag::Lower, mv, ply);
                return beta;
            }

            if score > alpha {
                alpha = score;
                best_move = mv;
                best_pv = child_pv;
                best_pv.insert(0, mv);
            }
        }

        if legal == 0 {
            return if in_check {
                -(MATE_SCORE - ply as i32)
            } else {
                DRAW_SCORE
            };
        }

        let flag = if alpha > orig_alpha { TtFlag::Exact } else { TtFlag::Upper };
        self.tt.store(current_hash, depth, alpha, flag, best_move, ply);

        *pv = best_pv;
        alpha
    }

    fn quiescence(&mut self, pos: &mut Position, mut alpha: i32, beta: i32) -> i32 {
        if self.stop.load(Ordering::Relaxed) { return 0; }
        self.nodes += 1;

        let stand_pat = evaluate(pos);
        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }

        let mut captures = MoveGen::generate_captures(pos);
        captures.sort_by_key(|m| -mvv_lva(pos, *m));

        for mv in captures {
            // Delta pruning: if even winning this capture (plus a safety margin)
            // can't raise us to alpha, it's hopeless — skip it. Never prune
            // promotions or en-passant, whose gain isn't the captured-square piece.
            if !mv.is_promotion() && !mv.is_ep() {
                let victim = pos.board[mv.to_sq() as usize]
                    .map(|p| p.kind.material_value())
                    .unwrap_or(0);
                if stand_pat + victim + 150 < alpha {
                    continue;
                }
            }

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

    fn order_moves(&self, pos: &Position, moves: &mut Vec<Move>, ply: usize, tt_move: Move) {
        moves.sort_by_key(|mv| {
            let mut score = 0i32;
            if !tt_move.is_null() && *mv == tt_move {
                score += 20_000;
            } else if mv.is_capture() {
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

/// True if `side` has at least one knight, bishop, rook, or queen. Used to
/// disable null-move pruning in likely-zugzwang king-and-pawn positions.
fn has_non_pawn_material(pos: &Position, side: Color) -> bool {
    let c = side as usize;
    // Piece bitboard index = kind*2 + color: knight=2+c, bishop=4+c, rook=6+c, queen=8+c.
    (pos.pieces[2 + c].0 | pos.pieces[4 + c].0 | pos.pieces[6 + c].0 | pos.pieces[8 + c].0) != 0
}

fn mvv_lva(pos: &Position, mv: Move) -> i32 {
    let attacker = pos.board[mv.from_sq() as usize].map(|p| p.kind.material_value()).unwrap_or(0);
    let victim   = pos.board[mv.to_sq() as usize].map(|p| p.kind.material_value()).unwrap_or(0);
    victim * 10 - attacker
}
