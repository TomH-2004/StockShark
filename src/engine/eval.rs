use crate::board::position::{Color, PieceType, Position};

/// Piece-square tables (from White's perspective, rank 1 at bottom)
/// Values in centipawns added to material score.

#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 35, 35, 20, 10, 10,  // rank 6: boost d/e advanced pawns
     5,  5, 10, 30, 30, 10,  5,  5,  // rank 5: d4/e4 now +30 (was 25)
     0,  0,  5, 30, 30,  5,  0,  0,  // rank 4: d5/e5 now +30 (was 20), makes pawn advance competitive with knight dev
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,  // rank 2: center start pawns get -20 (strong push incentive)
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

fn pst_value(pst: &[i32; 64], sq: u8, color: Color) -> i32 {
    // PSTs are written with rank 8 at index 0 (standard chess convention).
    // Our square numbering has sq=0 at a1 (rank 1), so White needs a rank flip.
    // Black promotes toward rank 1, so rank 7 (starting) maps to the same PST row as
    // White's rank 2, which is sq directly (no flip needed).
    let rank = sq / 8;
    let file = sq % 8;
    let idx = if color == Color::White {
        (7 - rank) as usize * 8 + file as usize
    } else {
        sq as usize
    };
    pst[idx]
}

/// Static evaluation from the perspective of the side to move.
/// Positive = good for side to move.
pub fn evaluate(pos: &Position) -> i32 {
    let mut score = 0i32;

    for sq in 0..64u8 {
        if let Some(piece) = pos.board[sq as usize] {
            let material = piece.kind.material_value();
            let positional = match piece.kind {
                PieceType::Pawn   => pst_value(&PST_PAWN,    sq, piece.color),
                PieceType::Knight => pst_value(&PST_KNIGHT,  sq, piece.color),
                PieceType::Bishop => pst_value(&PST_BISHOP,  sq, piece.color),
                PieceType::Rook   => pst_value(&PST_ROOK,    sq, piece.color),
                PieceType::Queen  => pst_value(&PST_QUEEN,   sq, piece.color),
                PieceType::King   => pst_value(&PST_KING_MG, sq, piece.color),
            };
            let value = material + positional;
            if piece.color == pos.side {
                score += value;
            } else {
                score -= value;
            }
        }
    }

    score
}

pub const MATE_SCORE: i32 = 100_000;
pub const DRAW_SCORE: i32 = 0;
