use crate::board::{
    bitboard::Bitboard,
    position::{
        Color, Move, PieceType, Position,
        FLAG_CAPTURE, FLAG_CASTLE_K, FLAG_CASTLE_Q, FLAG_DOUBLE_PUSH,
        FLAG_EP_CAPTURE, FLAG_QUIET, CR_WK, CR_WQ, CR_BK, CR_BQ,
    },
};

pub struct MoveGen;

impl MoveGen {
    pub fn generate(pos: &Position) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let us = pos.side as usize;
        let them = 1 - us;
        let our_occ = pos.occupancy[us];
        let their_occ = pos.occupancy[them];
        let all = pos.all_occupancy();

        Self::gen_pawns(pos, &mut moves, us, them, our_occ, their_occ, all);
        Self::gen_knights(pos, &mut moves, us, our_occ, their_occ);
        Self::gen_bishops(pos, &mut moves, us, our_occ, their_occ, all, false);
        Self::gen_rooks(pos, &mut moves, us, our_occ, their_occ, all, false);
        Self::gen_queens(pos, &mut moves, us, our_occ, their_occ, all);
        Self::gen_king(pos, &mut moves, us, our_occ, their_occ);
        Self::gen_castling(pos, &mut moves, us, all);

        moves
    }

    /// Generate only captures (for quiescence search)
    pub fn generate_captures(pos: &Position) -> Vec<Move> {
        let mut moves = Vec::with_capacity(32);
        let us = pos.side as usize;
        let them = 1 - us;
        let our_occ = pos.occupancy[us];
        let their_occ = pos.occupancy[them];
        let all = pos.all_occupancy();

        Self::gen_pawn_captures(pos, &mut moves, us, them, their_occ);
        Self::gen_knights_captures(pos, &mut moves, us, our_occ, their_occ);
        Self::gen_bishops_captures(pos, &mut moves, us, our_occ, their_occ, all, false);
        Self::gen_rooks_captures(pos, &mut moves, us, our_occ, their_occ, all, false);
        Self::gen_queens_captures(pos, &mut moves, us, our_occ, their_occ, all);
        Self::gen_king_captures(pos, &mut moves, us, our_occ, their_occ);

        moves
    }

    fn gen_pawns(
        pos: &Position, moves: &mut Vec<Move>,
        us: usize, _them: usize,
        _our_occ: Bitboard, their_occ: Bitboard, all: Bitboard,
    ) {
        let pawns = pos.pieces[PieceType::Pawn as usize * 2 + us];
        if us == Color::White as usize {
            // Single push
            let single = pawns.shift_north() & !all;
            let promo_rank = Bitboard(0xFF00000000000000);
            let normal_single = single & !promo_rank;
            let promos = single & promo_rank;

            for to in normal_single {
                moves.push(Move::new(to - 8, to, FLAG_QUIET));
            }
            // Double push from rank 2
            let double = (single & Bitboard(0x0000000000FF0000)).shift_north() & !all;
            for to in double {
                moves.push(Move::new(to - 16, to, FLAG_DOUBLE_PUSH));
            }
            // Promotions (quiet)
            for to in promos {
                Self::push_promotions(moves, to - 8, to, false);
            }
            // Captures
            let cap_e = pawns.shift_north().shift_east() & their_occ;
            let cap_w = pawns.shift_north().shift_west() & their_occ;
            let promo_cap_e = cap_e & promo_rank;
            let promo_cap_w = cap_w & promo_rank;
            for to in cap_e & !promo_rank { moves.push(Move::new(to - 9, to, FLAG_CAPTURE)); }
            for to in cap_w & !promo_rank { moves.push(Move::new(to - 7, to, FLAG_CAPTURE)); }
            for to in promo_cap_e { Self::push_promotions(moves, to - 9, to, true); }
            for to in promo_cap_w { Self::push_promotions(moves, to - 7, to, true); }
            // En passant
            if pos.ep_sq != 255 {
                let ep = Bitboard::from_square(pos.ep_sq);
                if !(pawns.shift_north().shift_east() & ep).is_empty() { moves.push(Move::new(pos.ep_sq - 9, pos.ep_sq, FLAG_EP_CAPTURE)); }
                if !(pawns.shift_north().shift_west() & ep).is_empty() { moves.push(Move::new(pos.ep_sq - 7, pos.ep_sq, FLAG_EP_CAPTURE)); }
            }
        } else {
            // Black pawns
            let single = pawns.shift_south() & !all;
            let promo_rank = Bitboard(0x00000000000000FF);
            let normal_single = single & !promo_rank;
            let promos = single & promo_rank;

            for to in normal_single {
                moves.push(Move::new(to + 8, to, FLAG_QUIET));
            }
            let double = (single & Bitboard(0x0000FF0000000000)).shift_south() & !all;
            for to in double {
                moves.push(Move::new(to + 16, to, FLAG_DOUBLE_PUSH));
            }
            for to in promos {
                Self::push_promotions(moves, to + 8, to, false);
            }
            let cap_e = pawns.shift_south().shift_east() & their_occ;
            let cap_w = pawns.shift_south().shift_west() & their_occ;
            let promo_cap_e = cap_e & promo_rank;
            let promo_cap_w = cap_w & promo_rank;
            for to in cap_e & !promo_rank { moves.push(Move::new(to + 7, to, FLAG_CAPTURE)); }
            for to in cap_w & !promo_rank { moves.push(Move::new(to + 9, to, FLAG_CAPTURE)); }
            for to in promo_cap_e { Self::push_promotions(moves, to + 7, to, true); }
            for to in promo_cap_w { Self::push_promotions(moves, to + 9, to, true); }
            if pos.ep_sq != 255 {
                let ep = Bitboard::from_square(pos.ep_sq);
                if !(pawns.shift_south().shift_east() & ep).is_empty() { moves.push(Move::new(pos.ep_sq + 7, pos.ep_sq, FLAG_EP_CAPTURE)); }
                if !(pawns.shift_south().shift_west() & ep).is_empty() { moves.push(Move::new(pos.ep_sq + 9, pos.ep_sq, FLAG_EP_CAPTURE)); }
            }
        }
    }

    fn gen_pawn_captures(
        pos: &Position, moves: &mut Vec<Move>,
        us: usize, _them: usize, their_occ: Bitboard,
    ) {
        let pawns = pos.pieces[PieceType::Pawn as usize * 2 + us];
        if us == Color::White as usize {
            let promo_rank = Bitboard(0xFF00000000000000);
            let cap_e = pawns.shift_north().shift_east() & their_occ;
            let cap_w = pawns.shift_north().shift_west() & their_occ;
            for to in cap_e & !promo_rank { moves.push(Move::new(to - 9, to, FLAG_CAPTURE)); }
            for to in cap_w & !promo_rank { moves.push(Move::new(to - 7, to, FLAG_CAPTURE)); }
            for to in cap_e & promo_rank { Self::push_promotions(moves, to - 9, to, true); }
            for to in cap_w & promo_rank { Self::push_promotions(moves, to - 7, to, true); }
            if pos.ep_sq != 255 {
                let ep = Bitboard::from_square(pos.ep_sq);
                if !(pawns.shift_north().shift_east() & ep).is_empty() { moves.push(Move::new(pos.ep_sq - 9, pos.ep_sq, FLAG_EP_CAPTURE)); }
                if !(pawns.shift_north().shift_west() & ep).is_empty() { moves.push(Move::new(pos.ep_sq - 7, pos.ep_sq, FLAG_EP_CAPTURE)); }
            }
        } else {
            let promo_rank = Bitboard(0x00000000000000FF);
            let cap_e = pawns.shift_south().shift_east() & their_occ;
            let cap_w = pawns.shift_south().shift_west() & their_occ;
            for to in cap_e & !promo_rank { moves.push(Move::new(to + 7, to, FLAG_CAPTURE)); }
            for to in cap_w & !promo_rank { moves.push(Move::new(to + 9, to, FLAG_CAPTURE)); }
            for to in cap_e & promo_rank { Self::push_promotions(moves, to + 7, to, true); }
            for to in cap_w & promo_rank { Self::push_promotions(moves, to + 9, to, true); }
            if pos.ep_sq != 255 {
                let ep = Bitboard::from_square(pos.ep_sq);
                if !(pawns.shift_south().shift_east() & ep).is_empty() { moves.push(Move::new(pos.ep_sq + 7, pos.ep_sq, FLAG_EP_CAPTURE)); }
                if !(pawns.shift_south().shift_west() & ep).is_empty() { moves.push(Move::new(pos.ep_sq + 9, pos.ep_sq, FLAG_EP_CAPTURE)); }
            }
        }
    }

    fn push_promotions(moves: &mut Vec<Move>, from: u8, to: u8, capture: bool) {
        moves.push(Move::new_promo(from, to, PieceType::Queen,  capture));
        moves.push(Move::new_promo(from, to, PieceType::Rook,   capture));
        moves.push(Move::new_promo(from, to, PieceType::Bishop, capture));
        moves.push(Move::new_promo(from, to, PieceType::Knight, capture));
    }

    fn gen_knights(pos: &Position, moves: &mut Vec<Move>, us: usize, our_occ: Bitboard, their_occ: Bitboard) {
        let knights = pos.pieces[PieceType::Knight as usize * 2 + us];
        for from in knights {
            let attacks = knight_attacks(from) & !our_occ;
            for to in attacks & their_occ { moves.push(Move::new(from, to, FLAG_CAPTURE)); }
            for to in attacks & !their_occ { moves.push(Move::new(from, to, FLAG_QUIET)); }
        }
    }

    fn gen_knights_captures(pos: &Position, moves: &mut Vec<Move>, us: usize, _our_occ: Bitboard, their_occ: Bitboard) {
        let knights = pos.pieces[PieceType::Knight as usize * 2 + us];
        for from in knights {
            for to in knight_attacks(from) & their_occ {
                moves.push(Move::new(from, to, FLAG_CAPTURE));
            }
        }
    }

    fn gen_bishops(pos: &Position, moves: &mut Vec<Move>, us: usize, our_occ: Bitboard, their_occ: Bitboard, all: Bitboard, queens_only: bool) {
        let bb = if queens_only {
            pos.pieces[PieceType::Queen as usize * 2 + us]
        } else {
            pos.pieces[PieceType::Bishop as usize * 2 + us]
        };
        for from in bb {
            let attacks = bishop_attacks(from, all) & !our_occ;
            for to in attacks & their_occ { moves.push(Move::new(from, to, FLAG_CAPTURE)); }
            for to in attacks & !their_occ { moves.push(Move::new(from, to, FLAG_QUIET)); }
        }
    }

    fn gen_bishops_captures(pos: &Position, moves: &mut Vec<Move>, us: usize, _our_occ: Bitboard, their_occ: Bitboard, all: Bitboard, queens_only: bool) {
        let bb = if queens_only {
            pos.pieces[PieceType::Queen as usize * 2 + us]
        } else {
            pos.pieces[PieceType::Bishop as usize * 2 + us]
        };
        for from in bb {
            for to in bishop_attacks(from, all) & their_occ {
                moves.push(Move::new(from, to, FLAG_CAPTURE));
            }
        }
    }

    fn gen_rooks(pos: &Position, moves: &mut Vec<Move>, us: usize, our_occ: Bitboard, their_occ: Bitboard, all: Bitboard, queens_only: bool) {
        let bb = if queens_only {
            pos.pieces[PieceType::Queen as usize * 2 + us]
        } else {
            pos.pieces[PieceType::Rook as usize * 2 + us]
        };
        for from in bb {
            let attacks = rook_attacks(from, all) & !our_occ;
            for to in attacks & their_occ { moves.push(Move::new(from, to, FLAG_CAPTURE)); }
            for to in attacks & !their_occ { moves.push(Move::new(from, to, FLAG_QUIET)); }
        }
    }

    fn gen_rooks_captures(pos: &Position, moves: &mut Vec<Move>, us: usize, _our_occ: Bitboard, their_occ: Bitboard, all: Bitboard, queens_only: bool) {
        let bb = if queens_only {
            pos.pieces[PieceType::Queen as usize * 2 + us]
        } else {
            pos.pieces[PieceType::Rook as usize * 2 + us]
        };
        for from in bb {
            for to in rook_attacks(from, all) & their_occ {
                moves.push(Move::new(from, to, FLAG_CAPTURE));
            }
        }
    }

    fn gen_queens(pos: &Position, moves: &mut Vec<Move>, us: usize, our_occ: Bitboard, their_occ: Bitboard, all: Bitboard) {
        let queens = pos.pieces[PieceType::Queen as usize * 2 + us];
        for from in queens {
            let attacks = (bishop_attacks(from, all) | rook_attacks(from, all)) & !our_occ;
            for to in attacks & their_occ { moves.push(Move::new(from, to, FLAG_CAPTURE)); }
            for to in attacks & !their_occ { moves.push(Move::new(from, to, FLAG_QUIET)); }
        }
    }

    fn gen_queens_captures(pos: &Position, moves: &mut Vec<Move>, us: usize, _our_occ: Bitboard, their_occ: Bitboard, all: Bitboard) {
        let queens = pos.pieces[PieceType::Queen as usize * 2 + us];
        for from in queens {
            for to in (bishop_attacks(from, all) | rook_attacks(from, all)) & their_occ {
                moves.push(Move::new(from, to, FLAG_CAPTURE));
            }
        }
    }

    fn gen_king(pos: &Position, moves: &mut Vec<Move>, us: usize, our_occ: Bitboard, their_occ: Bitboard) {
        let from = pos.king_sq[us];
        let attacks = king_attacks(from) & !our_occ;
        for to in attacks & their_occ { moves.push(Move::new(from, to, FLAG_CAPTURE)); }
        for to in attacks & !their_occ { moves.push(Move::new(from, to, FLAG_QUIET)); }
    }

    fn gen_king_captures(pos: &Position, moves: &mut Vec<Move>, us: usize, _our_occ: Bitboard, their_occ: Bitboard) {
        let from = pos.king_sq[us];
        for to in king_attacks(from) & their_occ {
            moves.push(Move::new(from, to, FLAG_CAPTURE));
        }
    }

    fn gen_castling(pos: &Position, moves: &mut Vec<Move>, us: usize, all: Bitboard) {
        let them = 1 - us;
        if us == Color::White as usize {
            // Can't castle out of check
            if Self::is_attacked(pos, 4, them) { return; }
            // Kingside: squares f1(5) and g1(6) must be empty; king passes through f1 so it must not be attacked
            if pos.castling & CR_WK != 0
                && (all & Bitboard(0x60)).is_empty()
                && !Self::is_attacked(pos, 5, them)
            {
                moves.push(Move::new(4, 6, FLAG_CASTLE_K));
            }
            // Queenside: squares b1-d1 must be empty; king passes through d1(3) so it must not be attacked
            if pos.castling & CR_WQ != 0
                && (all & Bitboard(0x0E)).is_empty()
                && !Self::is_attacked(pos, 3, them)
            {
                moves.push(Move::new(4, 2, FLAG_CASTLE_Q));
            }
        } else {
            if Self::is_attacked(pos, 60, them) { return; }
            if pos.castling & CR_BK != 0
                && (all & Bitboard(0x6000000000000000)).is_empty()
                && !Self::is_attacked(pos, 61, them)
            {
                moves.push(Move::new(60, 62, FLAG_CASTLE_K));
            }
            if pos.castling & CR_BQ != 0
                && (all & Bitboard(0x0E00000000000000)).is_empty()
                && !Self::is_attacked(pos, 59, them)
            {
                moves.push(Move::new(60, 58, FLAG_CASTLE_Q));
            }
        }
    }

    /// Check if a square is attacked by the given side
    pub fn is_attacked(pos: &Position, sq: u8, by: usize) -> bool {
        let all = pos.all_occupancy();
        let them_pawn = pos.pieces[PieceType::Pawn as usize * 2 + by];
        let them_knight = pos.pieces[PieceType::Knight as usize * 2 + by];
        let them_bishop = pos.pieces[PieceType::Bishop as usize * 2 + by];
        let them_rook = pos.pieces[PieceType::Rook as usize * 2 + by];
        let them_queen = pos.pieces[PieceType::Queen as usize * 2 + by];
        let them_king = pos.pieces[PieceType::King as usize * 2 + by];

        if !(knight_attacks(sq) & them_knight).is_empty() { return true; }
        if !(bishop_attacks(sq, all) & (them_bishop | them_queen)).is_empty() { return true; }
        if !(rook_attacks(sq, all) & (them_rook | them_queen)).is_empty() { return true; }
        if !(king_attacks(sq) & them_king).is_empty() { return true; }

        let sq_bb = Bitboard::from_square(sq);
        if by == Color::White as usize {
            // White pawns attack upward — sq attacked by white pawn if pawn is one rank below
            if !(sq_bb.shift_south().shift_east() & them_pawn).is_empty() { return true; }
            if !(sq_bb.shift_south().shift_west() & them_pawn).is_empty() { return true; }
        } else {
            if !(sq_bb.shift_north().shift_east() & them_pawn).is_empty() { return true; }
            if !(sq_bb.shift_north().shift_west() & them_pawn).is_empty() { return true; }
        }

        false
    }

    pub fn in_check(pos: &Position) -> bool {
        let us = pos.side as usize;
        let them = 1 - us;
        Self::is_attacked(pos, pos.king_sq[us], them)
    }
}

pub fn knight_attacks(sq: u8) -> Bitboard {
    static TABLE: std::sync::LazyLock<[Bitboard; 64]> = std::sync::LazyLock::new(|| {
        let mut t = [Bitboard::EMPTY; 64];
        for sq in 0..64u8 {
            let b = Bitboard::from_square(sq);
            t[sq as usize] =
                (b.shift_north().shift_north().shift_east()) |
                (b.shift_north().shift_north().shift_west()) |
                (b.shift_south().shift_south().shift_east()) |
                (b.shift_south().shift_south().shift_west()) |
                (b.shift_east().shift_east().shift_north()) |
                (b.shift_east().shift_east().shift_south()) |
                (b.shift_west().shift_west().shift_north()) |
                (b.shift_west().shift_west().shift_south());
        }
        t
    });
    TABLE[sq as usize]
}

pub fn king_attacks(sq: u8) -> Bitboard {
    static TABLE: std::sync::LazyLock<[Bitboard; 64]> = std::sync::LazyLock::new(|| {
        let mut t = [Bitboard::EMPTY; 64];
        for sq in 0..64u8 {
            let b = Bitboard::from_square(sq);
            t[sq as usize] =
                b.shift_north() | b.shift_south() | b.shift_east() | b.shift_west() |
                b.shift_north().shift_east() | b.shift_north().shift_west() |
                b.shift_south().shift_east() | b.shift_south().shift_west();
        }
        t
    });
    TABLE[sq as usize]
}

pub fn bishop_attacks(sq: u8, blockers: Bitboard) -> Bitboard {
    sliding_attacks(sq, blockers, &[9i32, 7, -7, -9])
}

pub fn rook_attacks(sq: u8, blockers: Bitboard) -> Bitboard {
    sliding_attacks(sq, blockers, &[8i32, -8, 1, -1])
}

fn sliding_attacks(sq: u8, blockers: Bitboard, dirs: &[i32]) -> Bitboard {
    let mut result = Bitboard::EMPTY;
    for &dir in dirs {
        let mut cur = sq as i32;
        loop {
            let prev_file = cur % 8;
            cur += dir;
            if cur < 0 || cur >= 64 { break; }
            let new_file = cur % 8;
            // Wrap-around guard: if file jumped more than 1, we crossed a board edge
            if (prev_file - new_file).abs() > 1 && (dir == 1 || dir == -1) { break; }
            if (prev_file - new_file).abs() > 1 && dir.abs() != 8 { break; }
            result.set(cur as u8);
            if blockers.contains(cur as u8) { break; }
        }
    }
    result
}
