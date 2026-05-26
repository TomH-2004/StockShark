use crate::board::position::{
    Color, Move, Piece, PieceType, Position,
    FLAG_CASTLE_K, FLAG_CASTLE_Q, FLAG_DOUBLE_PUSH,
    CR_WK, CR_WQ, CR_BK, CR_BQ,
};
use crate::board::zobrist;

/// Undo information saved before make_move
#[derive(Clone)]
pub struct Undo {
    pub castling: u8,
    pub ep_sq: u8,
    pub halfmove_clock: u32,
    pub hash: u64,
    pub captured: Option<Piece>,
}

pub fn make_move(pos: &mut Position, mv: Move) -> Undo {
    let from = mv.from_sq();
    let to   = mv.to_sq();
    let us   = pos.side as usize;
    let them = 1 - us;

    let undo = Undo {
        castling: pos.castling,
        ep_sq: pos.ep_sq,
        halfmove_clock: pos.halfmove_clock,
        hash: pos.hash,
        captured: pos.board[to as usize],
    };

    // Clear old EP key
    if pos.ep_sq != 255 {
        pos.hash ^= zobrist::ep_key(pos.ep_sq % 8);
        pos.ep_sq = 255;
    }

    let moving = pos.board[from as usize].unwrap();

    // Remove captured piece
    if mv.is_capture() && !mv.is_ep() {
        if let Some(captured) = pos.board[to as usize] {
            pos.pieces[captured.bb_index()].clear(to);
            pos.occupancy[them].clear(to);
            pos.hash ^= zobrist::piece_key(captured.bb_index(), to);
        }
    }

    // Move the piece
    pos.pieces[moving.bb_index()].clear(from);
    pos.occupancy[us].clear(from);
    pos.hash ^= zobrist::piece_key(moving.bb_index(), from);

    let mut landing_piece = moving;

    if mv.is_promotion() {
        landing_piece = Piece::new(mv.promo_piece(), moving.color);
    }

    pos.pieces[landing_piece.bb_index()].set(to);
    pos.occupancy[us].set(to);
    pos.hash ^= zobrist::piece_key(landing_piece.bb_index(), to);
    pos.board[from as usize] = None;
    pos.board[to as usize] = Some(landing_piece);

    if landing_piece.kind == PieceType::King {
        pos.king_sq[us] = to;
    }

    // En passant capture
    if mv.is_ep() {
        let ep_pawn_sq = if us == Color::White as usize { to - 8 } else { to + 8 };
        let their_pawn = Piece::new(PieceType::Pawn, if us == 0 { Color::Black } else { Color::White });
        pos.pieces[their_pawn.bb_index()].clear(ep_pawn_sq);
        pos.occupancy[them].clear(ep_pawn_sq);
        pos.hash ^= zobrist::piece_key(their_pawn.bb_index(), ep_pawn_sq);
        pos.board[ep_pawn_sq as usize] = None;
    }

    // Castling rook moves
    if mv.flags() == FLAG_CASTLE_K {
        let (rook_from, rook_to) = if us == 0 { (7u8, 5u8) } else { (63u8, 61u8) };
        move_piece(pos, rook_from, rook_to, us);
    } else if mv.flags() == FLAG_CASTLE_Q {
        let (rook_from, rook_to) = if us == 0 { (0u8, 3u8) } else { (56u8, 59u8) };
        move_piece(pos, rook_from, rook_to, us);
    }

    // Double pawn push sets EP square
    if mv.flags() == FLAG_DOUBLE_PUSH {
        pos.ep_sq = if us == Color::White as usize { to - 8 } else { to + 8 };
        pos.hash ^= zobrist::ep_key(pos.ep_sq % 8);
    }

    // Update castling rights
    pos.hash ^= zobrist::castling_key(pos.castling);
    update_castling(pos, from, to);
    pos.hash ^= zobrist::castling_key(pos.castling);

    // Halfmove clock
    if moving.kind == PieceType::Pawn || mv.is_capture() {
        pos.halfmove_clock = 0;
    } else {
        pos.halfmove_clock += 1;
    }

    if us == Color::Black as usize {
        pos.fullmove += 1;
    }

    pos.side = pos.side.flip();
    pos.hash ^= zobrist::side_key();

    undo
}

pub fn unmake_move(pos: &mut Position, mv: Move, undo: Undo) {
    pos.side = pos.side.flip();
    let us   = pos.side as usize;
    let them = 1 - us;

    let from = mv.from_sq();
    let to   = mv.to_sq();

    let landing_piece = pos.board[to as usize].unwrap();

    // Move piece back
    pos.pieces[landing_piece.bb_index()].clear(to);
    pos.occupancy[us].clear(to);
    pos.board[to as usize] = None;

    // If promotion, restore pawn
    let original_piece = if mv.is_promotion() {
        Piece::new(PieceType::Pawn, landing_piece.color)
    } else {
        landing_piece
    };

    pos.pieces[original_piece.bb_index()].set(from);
    pos.occupancy[us].set(from);
    pos.board[from as usize] = Some(original_piece);

    if original_piece.kind == PieceType::King {
        pos.king_sq[us] = from;
    }

    // Restore capture
    if mv.is_capture() && !mv.is_ep() {
        if let Some(captured) = undo.captured {
            pos.pieces[captured.bb_index()].set(to);
            pos.occupancy[them].set(to);
            pos.board[to as usize] = Some(captured);
        }
    }

    // Restore EP captured pawn
    if mv.is_ep() {
        let ep_pawn_sq = if us == Color::White as usize { to - 8 } else { to + 8 };
        let their_pawn = Piece::new(PieceType::Pawn, if us == 0 { Color::Black } else { Color::White });
        pos.pieces[their_pawn.bb_index()].set(ep_pawn_sq);
        pos.occupancy[them].set(ep_pawn_sq);
        pos.board[ep_pawn_sq as usize] = Some(their_pawn);
    }

    // Undo castling
    if mv.flags() == FLAG_CASTLE_K {
        let (rook_from, rook_to) = if us == 0 { (7u8, 5u8) } else { (63u8, 61u8) };
        move_piece(pos, rook_to, rook_from, us);
    } else if mv.flags() == FLAG_CASTLE_Q {
        let (rook_from, rook_to) = if us == 0 { (0u8, 3u8) } else { (56u8, 59u8) };
        move_piece(pos, rook_to, rook_from, us);
    }

    pos.castling = undo.castling;
    pos.ep_sq = undo.ep_sq;
    pos.halfmove_clock = undo.halfmove_clock;
    pos.hash = undo.hash;

    if us == Color::Black as usize {
        pos.fullmove -= 1;
    }
}

fn move_piece(pos: &mut Position, from: u8, to: u8, us: usize) {
    if let Some(piece) = pos.board[from as usize] {
        pos.pieces[piece.bb_index()].clear(from);
        pos.occupancy[us].clear(from);
        pos.hash ^= zobrist::piece_key(piece.bb_index(), from);

        pos.pieces[piece.bb_index()].set(to);
        pos.occupancy[us].set(to);
        pos.hash ^= zobrist::piece_key(piece.bb_index(), to);

        pos.board[from as usize] = None;
        pos.board[to as usize] = Some(piece);
    }
}

fn update_castling(pos: &mut Position, from: u8, to: u8) {
    const CASTLING_MASK: [u8; 64] = {
        let mut m = [0xFFu8; 64];
        m[0]  &= !CR_WQ;
        m[4]  &= !(CR_WK | CR_WQ);
        m[7]  &= !CR_WK;
        m[56] &= !CR_BQ;
        m[60] &= !(CR_BK | CR_BQ);
        m[63] &= !CR_BK;
        m
    };
    pos.castling &= CASTLING_MASK[from as usize] & CASTLING_MASK[to as usize];
}
