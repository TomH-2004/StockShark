use crate::board::position::{Color, PieceType, Position};

/// Piece-square tables (from White's perspective, rank 1 at bottom)
/// Values in centipawns added to material score.

#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 35, 35, 20, 10, 10,
     5,  5, 10, 30, 30, 10,  5,  5,
     0,  0,  5, 30, 30,  5,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const PST_ROOK: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];

#[rustfmt::skip]
const PST_QUEEN: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

// Middlegame king: stay castled, avoid centre.
#[rustfmt::skip]
const PST_KING_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];

// Endgame king: centralise; heavy penalty on edges/corners.
#[rustfmt::skip]
const PST_KING_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
];

fn pst_value(pst: &[i32; 64], sq: u8, color: Color) -> i32 {
    let rank = sq / 8;
    let file = sq % 8;
    let idx = if color == Color::White {
        (7 - rank) as usize * 8 + file as usize
    } else {
        sq as usize
    };
    pst[idx]
}

/// Returns true if neither side can force checkmate with the material on board.
pub fn is_insufficient_material(pos: &Position) -> bool {
    let mut wn = 0u8;
    let mut bn = 0u8;
    let mut wb = 0u8; // white bishops
    let mut bb = 0u8; // black bishops
    let mut w_bishop_light = false; // square colour of the last white bishop seen
    let mut b_bishop_light = false;

    for sq in 0..64u8 {
        if let Some(p) = pos.board[sq as usize] {
            match p.kind {
                PieceType::King => {}
                PieceType::Knight => {
                    if p.color == Color::White { wn += 1; } else { bn += 1; }
                }
                PieceType::Bishop => {
                    // "light" = (file+rank) % 2 == 1 — just needs to be consistent.
                    let light = (sq % 8 + sq / 8) % 2 == 1;
                    if p.color == Color::White {
                        wb += 1;
                        w_bishop_light = light;
                    } else {
                        bb += 1;
                        b_bishop_light = light;
                    }
                }
                // Any pawn, rook, or queen means checkmate is possible.
                _ => return false,
            }
        }
    }

    // K vs K
    if wn + wb + bn + bb == 0 { return true; }
    // K+minor vs K  (one side has exactly one knight or one bishop, other has nothing)
    if wn + wb <= 1 && bn + bb == 0 { return true; }
    if bn + bb <= 1 && wn + wb == 0 { return true; }
    // K+B vs K+B, both bishops on the same colour square
    if wb == 1 && wn == 0 && bb == 1 && bn == 0 && w_bishop_light == b_bishop_light {
        return true;
    }

    false
}

fn non_king_material(pos: &Position) -> i32 {
    pos.board
        .iter()
        .filter_map(|p| *p)
        .filter(|p| p.kind != PieceType::King)
        .map(|p| p.kind.material_value())
        .sum()
}

/// Chebyshev (king-move) distance between two squares.
fn chebyshev(sq1: u8, sq2: u8) -> i32 {
    let df = ((sq1 % 8) as i32 - (sq2 % 8) as i32).abs();
    let dr = ((sq1 / 8) as i32 - (sq2 / 8) as i32).abs();
    df.max(dr)
}

/// How far a square is from the nearest board edge (0 = on the edge).
fn edge_dist(sq: u8) -> i32 {
    let f = (sq % 8) as i32;
    let r = (sq / 8) as i32;
    f.min(7 - f).min(r).min(7 - r)
}

/// Static evaluation from the perspective of the side to move.
/// Positive = good for side to move.
pub fn evaluate(pos: &Position) -> i32 {
    if is_insufficient_material(pos) {
        return DRAW_SCORE;
    }

    let total_mat = non_king_material(pos);
    let endgame = total_mat <= 2000;

    // ── Build pawn-file rank bitmasks ─────────────────────────────────────────
    // wpawn[f] has bit r set when White has a pawn on (file=f, rank=r).
    let mut wpawn = [0u8; 8];
    let mut bpawn = [0u8; 8];
    for sq in 0..64u8 {
        if let Some(p) = pos.board[sq as usize] {
            if p.kind == PieceType::Pawn {
                let f = (sq % 8) as usize;
                let r = sq / 8;
                match p.color {
                    Color::White => wpawn[f] |= 1 << r,
                    Color::Black => bpawn[f] |= 1 << r,
                }
            }
        }
    }

    let mut score = 0i32;
    let mut white_mat = 0i32;
    let mut black_mat = 0i32;
    let mut white_bishops = 0u8;
    let mut black_bishops = 0u8;

    for sq in 0..64u8 {
        let Some(piece) = pos.board[sq as usize] else { continue };

        let f = (sq % 8) as usize;
        let r = (sq / 8) as usize;

        let material = piece.kind.material_value();
        let positional = match piece.kind {
            PieceType::Pawn   => pst_value(&PST_PAWN,   sq, piece.color),
            PieceType::Knight => pst_value(&PST_KNIGHT, sq, piece.color),
            PieceType::Bishop => pst_value(&PST_BISHOP, sq, piece.color),
            PieceType::Rook   => pst_value(&PST_ROOK,   sq, piece.color),
            PieceType::Queen  => pst_value(&PST_QUEEN,  sq, piece.color),
            PieceType::King   => {
                if endgame { pst_value(&PST_KING_EG, sq, piece.color) }
                else       { pst_value(&PST_KING_MG, sq, piece.color) }
            }
        };

        let mut structural = 0i32;

        match piece.kind {
            PieceType::Pawn => {
                let (own, opp) = match piece.color {
                    Color::White => (&wpawn, &bpawn),
                    Color::Black => (&bpawn, &wpawn),
                };

                // Doubled pawn penalty (more than one own pawn on this file).
                if own[f].count_ones() > 1 { structural -= 15; }

                // Isolated pawn penalty (no own pawns on adjacent files).
                let has_neighbour = (f > 0 && own[f - 1] != 0) || (f < 7 && own[f + 1] != 0);
                if !has_neighbour { structural -= 20; }

                // Passed pawn bonus.  A pawn is passed when no opponent pawn can
                // ever block or capture it: no opp pawn on the same or adjacent
                // files in the squares ahead of this pawn.
                let (ahead_mask, advancement) = match piece.color {
                    Color::White => {
                        let m = if r < 7 { 0xFF_u8 << (r + 1) } else { 0 };
                        (m, r as i32) // r=1 = rank 2 (just started), r=6 = rank 7
                    }
                    Color::Black => {
                        let m = if r > 0 { (1u8 << r).wrapping_sub(1) } else { 0 };
                        (m, (7 - r) as i32) // higher = more advanced toward rank 1
                    }
                };
                let fmin = f.saturating_sub(1);
                let fmax = (f + 1).min(7);
                let blockers = opp[fmin] | opp[f] | opp[fmax];
                if blockers & ahead_mask == 0 {
                    // Bonus grows the further advanced the passed pawn is.
                    structural += 15 + (advancement - 1).max(0) * 20;
                }
            }

            PieceType::Rook => {
                let (own, opp) = match piece.color {
                    Color::White => (&wpawn, &bpawn),
                    Color::Black => (&bpawn, &wpawn),
                };
                if own[f] == 0 {
                    // Open file (no pawns at all) or semi-open (only opponent pawns).
                    structural += if opp[f] == 0 { 20 } else { 10 };
                }
            }

            PieceType::Bishop => {
                match piece.color {
                    Color::White => white_bishops += 1,
                    Color::Black => black_bishops += 1,
                }
            }

            _ => {}
        }

        let value = material + positional + structural;
        if piece.color == pos.side {
            score += value;
        } else {
            score -= value;
        }

        if piece.kind != PieceType::King {
            match piece.color {
                Color::White => white_mat += material,
                Color::Black => black_mat += material,
            }
        }
    }

    // ── Bishop pair bonus ─────────────────────────────────────────────────────
    if white_bishops >= 2 {
        if pos.side == Color::White { score += 30; } else { score -= 30; }
    }
    if black_bishops >= 2 {
        if pos.side == Color::Black { score += 30; } else { score -= 30; }
    }

    // ── Endgame king-coordination bonus ──────────────────────────────────────
    // When one side has more material, reward the attacking king for being
    // close to the defending king and the defending king for being near an edge.
    if endgame && white_mat != black_mat {
        let wk = pos.king_sq[Color::White as usize];
        let bk = pos.king_sq[Color::Black as usize];

        let (attk_king, def_king) = if white_mat > black_mat { (wk, bk) } else { (bk, wk) };

        let dist  = chebyshev(attk_king, def_king);
        let dedge = edge_dist(def_king);
        // Up to 60 cp for kings adjacent; up to 60 cp for def king cornered.
        let bonus = (7 - dist) * 10 + (3 - dedge.min(3)) * 20;

        let winning = if white_mat > black_mat { Color::White } else { Color::Black };
        if pos.side == winning { score += bonus; } else { score -= bonus; }
    }

    score
}

pub const MATE_SCORE: i32 = 100_000;
pub const DRAW_SCORE: i32 = 0;
