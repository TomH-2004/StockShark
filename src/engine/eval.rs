use crate::board::position::{Color, PieceType, Position};
use crate::moves::generator::{bishop_attacks, knight_attacks, rook_attacks};

// ── PeSTO tapered evaluation ──────────────────────────────────────────────────
// Material and piece-square tables from the PeSTO evaluation (Ronald Friederich,
// Texel-tuned; public domain). Two sets of values — middlegame (mg) and endgame
// (eg) — are blended by a game-phase weight so the score transitions smoothly
// instead of switching abruptly. Tables are written rank-8-first (index 0 = a8),
// from White's perspective, matching `pst_value`'s orientation.

// Base piece values, folded into the tapered score alongside the PST.
const MG_VALUE: [i32; 6] = [82, 337, 365, 477, 1025, 0];
const EG_VALUE: [i32; 6] = [94, 281, 297, 512, 936, 0];

// Phase weights per piece (P, N, B, R, Q, K); full board = 24.
const PHASE_WEIGHT: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

#[rustfmt::skip]
const MG_PAWN: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
     98, 134,  61,  95,  68, 126,  34, -11,
     -6,   7,  26,  31,  65,  56,  25, -20,
    -14,  13,   6,  21,  23,  12,  17, -23,
    -27,  -2,  -5,  12,  17,   6,  10, -25,
    -26,  -4,  -4, -10,   3,   3,  33, -12,
    -35,  -1, -20, -23, -15,  24,  38, -22,
      0,   0,   0,   0,   0,   0,   0,   0,
];
#[rustfmt::skip]
const EG_PAWN: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8,  10,  13,   0,   2,  -7,
      0,   0,   0,   0,   0,   0,   0,   0,
];
#[rustfmt::skip]
const MG_KNIGHT: [i32; 64] = [
   -167, -89, -34, -49,  61, -97, -15,-107,
    -73, -41,  72,  36,  23,  62,   7, -17,
    -47,  60,  37,  65,  84, 129,  73,  44,
     -9,  17,  19,  53,  37,  69,  18,  22,
    -13,   4,  16,  13,  28,  19,  21,  -8,
    -23,  -9,  12,  10,  19,  17,  25, -16,
    -29, -53, -12,  -3,  -1,  18, -14, -19,
   -105, -21, -58, -33, -17, -28, -19, -23,
];
#[rustfmt::skip]
const EG_KNIGHT: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];
#[rustfmt::skip]
const MG_BISHOP: [i32; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];
#[rustfmt::skip]
const EG_BISHOP: [i32; 64] = [
    -14, -21, -11,  -8,  -7,  -9, -17, -24,
     -8,  -4,   7, -12,  -3, -13,  -4, -14,
      2,  -8,   0,  -1,  -2,   6,   0,   4,
     -3,   9,  12,   9,  14,  10,   3,   2,
     -6,   3,  13,  19,   7,  10,  -3,  -9,
    -12,  -3,   8,  10,  13,   3,  -7, -15,
    -14, -18,  -7,  -1,   4,  -9, -15, -27,
    -23,  -9, -23,  -5,  -9, -16,  -5, -17,
];
#[rustfmt::skip]
const MG_ROOK: [i32; 64] = [
     32,  42,  32,  51,  63,   9,  31,  43,
     27,  32,  58,  62,  80,  67,  26,  44,
     -5,  19,  26,  36,  17,  45,  61,  16,
    -24, -11,   7,  26,  24,  35,  -8, -20,
    -36, -26, -12,  -1,   9,  -7,   6, -23,
    -45, -25, -16, -17,   3,   0,  -5, -33,
    -44, -16, -20,  -9,  -1,  11,  -6, -71,
    -19, -13,   1,  17,  16,   7, -37, -26,
];
#[rustfmt::skip]
const EG_ROOK: [i32; 64] = [
     13,  10,  18,  15,  12,  12,   8,   5,
     11,  13,  13,  11,  -3,   3,   8,   3,
      7,   7,   7,   5,   4,  -3,  -5,  -3,
      4,   3,  13,   1,   2,   1,  -1,   2,
      3,   5,   8,   4,  -5,  -6,  -8, -11,
     -4,   0,  -5,  -1,  -7, -12,  -8, -16,
     -6,  -6,   0,   2,  -9,  -9, -11,  -3,
     -9,   2,   3,  -1,  -5, -13,   4, -20,
];
#[rustfmt::skip]
const MG_QUEEN: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];
#[rustfmt::skip]
const EG_QUEEN: [i32; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];
#[rustfmt::skip]
const MG_KING: [i32; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];
#[rustfmt::skip]
const EG_KING: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

fn mg_table(kind: PieceType) -> &'static [i32; 64] {
    match kind {
        PieceType::Pawn => &MG_PAWN,
        PieceType::Knight => &MG_KNIGHT,
        PieceType::Bishop => &MG_BISHOP,
        PieceType::Rook => &MG_ROOK,
        PieceType::Queen => &MG_QUEEN,
        PieceType::King => &MG_KING,
    }
}
fn eg_table(kind: PieceType) -> &'static [i32; 64] {
    match kind {
        PieceType::Pawn => &EG_PAWN,
        PieceType::Knight => &EG_KNIGHT,
        PieceType::Bishop => &EG_BISHOP,
        PieceType::Rook => &EG_ROOK,
        PieceType::Queen => &EG_QUEEN,
        PieceType::King => &EG_KING,
    }
}

/// Index into a rank-8-first, White's-perspective table for a piece of `color`
/// standing on `sq`. Black reads the vertically-mirrored square.
fn pst_index(sq: u8, color: Color) -> usize {
    let rank = sq / 8;
    let file = sq % 8;
    if color == Color::White {
        (7 - rank) as usize * 8 + file as usize
    } else {
        sq as usize
    }
}

fn pst_value(pst: &[i32; 64], sq: u8, color: Color) -> i32 {
    pst[pst_index(sq, color)]
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

/// King-safety score for `color` (higher = safer): a pawn shield in front of the
/// king. Cheap (no attack generation) so it doesn't slow the per-node eval.
/// Returned untapered; the caller fades it out toward the endgame.
fn king_safety(pos: &Position, color: Color) -> i32 {
    let c = color as usize;
    let ksq = pos.king_sq[c];
    let kf = (ksq % 8) as i32;
    let kr = (ksq / 8) as i32;
    let own_pawns = pos.pieces[PieceType::Pawn as usize * 2 + c];

    // Pawn shield: friendly pawns one or two ranks in front of the king, on its
    // own file and the two neighbours. A present shield pawn helps; a gap hurts.
    let (r1, r2) = match color {
        Color::White => (kr + 1, kr + 2),
        Color::Black => (kr - 1, kr - 2),
    };
    let mut shield = 0i32;
    for df in -1..=1 {
        let f = kf + df;
        if !(0..=7).contains(&f) { continue; }
        let mut has_pawn = false;
        for rr in [r1, r2] {
            if (0..=7).contains(&rr) && own_pawns.contains((rr * 8 + f) as u8) {
                has_pawn = true;
                break;
            }
        }
        if has_pawn { shield += 8; } else { shield -= 12; }
    }

    shield
}

/// Static evaluation from the perspective of the side to move.
/// Positive = good for side to move.
pub fn evaluate(pos: &Position) -> i32 {
    if is_insufficient_material(pos) {
        return DRAW_SCORE;
    }

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

    // Tapered material+PST accumulators and the structural adjustment, all kept
    // from White's perspective, then flipped to side-to-move at the end.
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut phase = 0i32;
    let mut structural = 0i32;
    let mut white_bishops = 0u8;
    let mut black_bishops = 0u8;
    let mut white_mat = 0i32;
    let mut black_mat = 0i32;
    let all = pos.all_occupancy();

    for sq in 0..64u8 {
        let Some(piece) = pos.board[sq as usize] else { continue };

        let f = (sq % 8) as usize;
        let r = (sq / 8) as usize;
        let k = piece.kind as usize;
        let idx = pst_index(sq, piece.color);

        let piece_mg = MG_VALUE[k] + mg_table(piece.kind)[idx];
        let piece_eg = EG_VALUE[k] + eg_table(piece.kind)[idx];
        phase += PHASE_WEIGHT[k];

        if piece.color == Color::White {
            mg += piece_mg;
            eg += piece_eg;
        } else {
            mg -= piece_mg;
            eg -= piece_eg;
        }

        // ── Mobility: reachable squares (incl. captures), centered so a
        // typically-placed piece scores ~0, then weighted by piece type. ──────
        let own = pos.occupancy[piece.color as usize];
        let mob = match piece.kind {
            PieceType::Knight => (knight_attacks(sq) & !own).count() as i32,
            PieceType::Bishop => (bishop_attacks(sq, all) & !own).count() as i32,
            PieceType::Rook   => (rook_attacks(sq, all) & !own).count() as i32,
            PieceType::Queen  => ((bishop_attacks(sq, all) | rook_attacks(sq, all)) & !own).count() as i32,
            _ => -1, // sentinel: no mobility term for pawns/king
        };
        let mob_bonus = match piece.kind {
            PieceType::Knight => (mob - 4) * 4,
            PieceType::Bishop => (mob - 6) * 4,
            PieceType::Rook   => (mob - 7) * 3,
            PieceType::Queen  => (mob - 13) * 2,
            _ => 0,
        };

        // ── Structural terms (untapered, small) — kept from White's view ──────
        let mut s = mob_bonus;
        match piece.kind {
            PieceType::Pawn => {
                let (own, opp) = match piece.color {
                    Color::White => (&wpawn, &bpawn),
                    Color::Black => (&bpawn, &wpawn),
                };
                if own[f].count_ones() > 1 { s -= 15; }
                let has_neighbour = (f > 0 && own[f - 1] != 0) || (f < 7 && own[f + 1] != 0);
                if !has_neighbour { s -= 20; }

                let (ahead_mask, advancement) = match piece.color {
                    Color::White => {
                        let m = if r < 7 { 0xFF_u8 << (r + 1) } else { 0 };
                        (m, r as i32)
                    }
                    Color::Black => {
                        let m = if r > 0 { (1u8 << r).wrapping_sub(1) } else { 0 };
                        (m, (7 - r) as i32)
                    }
                };
                let fmin = f.saturating_sub(1);
                let fmax = (f + 1).min(7);
                let blockers = opp[fmin] | opp[f] | opp[fmax];
                if blockers & ahead_mask == 0 {
                    s += 15 + (advancement - 1).max(0) * 20;
                }
            }
            PieceType::Rook => {
                let (own, opp) = match piece.color {
                    Color::White => (&wpawn, &bpawn),
                    Color::Black => (&bpawn, &wpawn),
                };
                if own[f] == 0 {
                    s += if opp[f] == 0 { 20 } else { 10 };
                }
            }
            PieceType::Bishop => match piece.color {
                Color::White => white_bishops += 1,
                Color::Black => black_bishops += 1,
            },
            _ => {}
        }
        if piece.color == Color::White { structural += s; } else { structural -= s; }

        if piece.kind != PieceType::King {
            match piece.color {
                Color::White => white_mat += piece.kind.material_value(),
                Color::Black => black_mat += piece.kind.material_value(),
            }
        }
    }

    // Bishop pair (White's perspective).
    if white_bishops >= 2 { structural += 30; }
    if black_bishops >= 2 { structural -= 30; }

    // ── Taper material+PST by game phase ─────────────────────────────────────
    let phase = phase.min(MAX_PHASE);
    let mut score = (mg * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE;
    score += structural;

    // King safety matters most with queens/rooks on the board, so fade it out
    // toward the endgame in proportion to the game phase.
    let king_safety_diff = king_safety(pos, Color::White) - king_safety(pos, Color::Black);
    score += king_safety_diff * phase / MAX_PHASE;

    // ── Endgame king-coordination bonus (drive the loser's king to the edge) ──
    let total_mat = non_king_material(pos);
    if total_mat <= 2000 && white_mat != black_mat {
        let wk = pos.king_sq[Color::White as usize];
        let bk = pos.king_sq[Color::Black as usize];
        let (attk_king, def_king) = if white_mat > black_mat { (wk, bk) } else { (bk, wk) };
        let dist = chebyshev(attk_king, def_king);
        let dedge = edge_dist(def_king);
        let bonus = (7 - dist) * 10 + (3 - dedge.min(3)) * 20;
        if white_mat > black_mat { score += bonus; } else { score -= bonus; }
    }

    // Flip White's-perspective score to the side to move.
    if pos.side == Color::White { score } else { -score }
}

pub const MATE_SCORE: i32 = 100_000;
pub const DRAW_SCORE: i32 = 0;
