use super::bitboard::Bitboard;
use super::zobrist;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub fn flip(self) -> Self {
        match self { Color::White => Color::Black, Color::Black => Color::White }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    pub fn material_value(self) -> i32 {
        match self {
            PieceType::Pawn   => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook   => 500,
            PieceType::Queen  => 900,
            PieceType::King   => 20000,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub kind: PieceType,
    pub color: Color,
}

impl Piece {
    pub fn new(kind: PieceType, color: Color) -> Self { Self { kind, color } }
    /// Index into piece bitboard array: kind*2 + color
    pub fn bb_index(self) -> usize {
        self.kind as usize * 2 + self.color as usize
    }
}

/// Encoded move: from(6) | to(6) | promo(3) | flags(4)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Move(pub u32);

pub const FLAG_QUIET:      u32 = 0;
pub const FLAG_DOUBLE_PUSH: u32 = 1;
pub const FLAG_CASTLE_K:   u32 = 2;
pub const FLAG_CASTLE_Q:   u32 = 3;
pub const FLAG_CAPTURE:    u32 = 4;
pub const FLAG_EP_CAPTURE: u32 = 5;
pub const FLAG_PROMO:      u32 = 8;
pub const FLAG_PROMO_CAP:  u32 = 12;

impl Move {
    pub fn new(from: u8, to: u8, flags: u32) -> Self {
        Self((from as u32) | ((to as u32) << 6) | (flags << 12))
    }

    pub fn new_promo(from: u8, to: u8, promo: PieceType, capture: bool) -> Self {
        let flag = if capture { FLAG_PROMO_CAP } else { FLAG_PROMO } | (promo as u32 - 1);
        Self((from as u32) | ((to as u32) << 6) | (flag << 12))
    }

    pub fn from_sq(self) -> u8 { (self.0 & 0x3F) as u8 }
    pub fn to_sq(self) -> u8   { ((self.0 >> 6) & 0x3F) as u8 }
    pub fn flags(self) -> u32  { (self.0 >> 12) & 0xF }

    pub fn is_capture(self) -> bool { self.flags() & FLAG_CAPTURE != 0 }
    pub fn is_ep(self) -> bool { self.flags() == FLAG_EP_CAPTURE }
    pub fn is_castle(self) -> bool {
        self.flags() == FLAG_CASTLE_K || self.flags() == FLAG_CASTLE_Q
    }
    pub fn is_promotion(self) -> bool { self.flags() & FLAG_PROMO != 0 }

    pub fn promo_piece(self) -> PieceType {
        match (self.flags() & 3) + 1 {
            1 => PieceType::Knight,
            2 => PieceType::Bishop,
            3 => PieceType::Rook,
            _ => PieceType::Queen,
        }
    }

    pub fn is_null(self) -> bool { self.0 == 0 }

    /// UCI string e.g. "e2e4", "e7e8q"
    pub fn to_uci(self) -> String {
        let from = sq_to_str(self.from_sq());
        let to   = sq_to_str(self.to_sq());
        if self.is_promotion() {
            let p = match self.promo_piece() {
                PieceType::Queen  => 'q',
                PieceType::Rook   => 'r',
                PieceType::Bishop => 'b',
                PieceType::Knight => 'n',
                _ => 'q',
            };
            format!("{}{}{}", from, to, p)
        } else {
            format!("{}{}", from, to)
        }
    }
}

pub fn sq_to_str(sq: u8) -> String {
    let file = b'a' + (sq % 8);
    let rank = b'1' + (sq / 8);
    format!("{}{}", file as char, rank as char)
}

pub fn str_to_sq(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    if b.len() < 2 { return None; }
    let file = b[0].wrapping_sub(b'a');
    let rank = b[1].wrapping_sub(b'1');
    if file > 7 || rank > 7 { return None; }
    Some(rank * 8 + file)
}

/// Castling rights bitmask: bit0=WK, bit1=WQ, bit2=BK, bit3=BQ
pub const CR_WK: u8 = 1;
pub const CR_WQ: u8 = 2;
pub const CR_BK: u8 = 4;
pub const CR_BQ: u8 = 8;

#[derive(Clone, Debug)]
pub struct Position {
    /// Bitboards indexed by Piece::bb_index()
    pub pieces: [Bitboard; 12],
    /// Combined occupancy per color
    pub occupancy: [Bitboard; 2],
    pub side: Color,
    pub castling: u8,
    /// En passant target square (255 = none)
    pub ep_sq: u8,
    pub halfmove_clock: u32,
    pub fullmove: u32,
    pub hash: u64,
    /// Square of each king for fast check detection
    pub king_sq: [u8; 2],
    /// Board array for O(1) piece lookup by square
    pub board: [Option<Piece>; 64],
}

impl Position {
    /// Standard starting position
    pub fn startpos() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return None; }

        let mut pos = Position {
            pieces: [Bitboard::EMPTY; 12],
            occupancy: [Bitboard::EMPTY; 2],
            side: Color::White,
            castling: 0,
            ep_sq: 255,
            halfmove_clock: 0,
            fullmove: 1,
            hash: 0,
            king_sq: [255; 2],
            board: [None; 64],
        };

        // Parse piece placement
        let mut sq: i32 = 56; // start at a8
        for ch in parts[0].chars() {
            match ch {
                '/' => sq -= 16,
                '1'..='8' => sq += ch as i32 - '0' as i32,
                _ => {
                    let (color, kind) = match ch {
                        'P' => (Color::White, PieceType::Pawn),
                        'N' => (Color::White, PieceType::Knight),
                        'B' => (Color::White, PieceType::Bishop),
                        'R' => (Color::White, PieceType::Rook),
                        'Q' => (Color::White, PieceType::Queen),
                        'K' => (Color::White, PieceType::King),
                        'p' => (Color::Black, PieceType::Pawn),
                        'n' => (Color::Black, PieceType::Knight),
                        'b' => (Color::Black, PieceType::Bishop),
                        'r' => (Color::Black, PieceType::Rook),
                        'q' => (Color::Black, PieceType::Queen),
                        'k' => (Color::Black, PieceType::King),
                        _ => return None,
                    };
                    let piece = Piece::new(kind, color);
                    pos.pieces[piece.bb_index()].set(sq as u8);
                    pos.occupancy[color as usize].set(sq as u8);
                    pos.board[sq as usize] = Some(piece);
                    pos.hash ^= zobrist::piece_key(piece.bb_index(), sq as u8);
                    if kind == PieceType::King {
                        pos.king_sq[color as usize] = sq as u8;
                    }
                    sq += 1;
                }
            }
        }

        pos.side = if parts[1] == "b" { Color::Black } else { Color::White };
        if pos.side == Color::Black { pos.hash ^= zobrist::side_key(); }

        for ch in parts[2].chars() {
            match ch {
                'K' => pos.castling |= CR_WK,
                'Q' => pos.castling |= CR_WQ,
                'k' => pos.castling |= CR_BK,
                'q' => pos.castling |= CR_BQ,
                _ => {}
            }
        }
        pos.hash ^= zobrist::castling_key(pos.castling);

        if parts[3] != "-" {
            pos.ep_sq = str_to_sq(parts[3]).unwrap_or(255);
            if pos.ep_sq != 255 {
                pos.hash ^= zobrist::ep_key(pos.ep_sq % 8);
            }
        }

        if parts.len() > 4 { pos.halfmove_clock = parts[4].parse().unwrap_or(0); }
        if parts.len() > 5 { pos.fullmove = parts[5].parse().unwrap_or(1); }

        Some(pos)
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0u8;
            for file in 0..8 {
                let sq = rank * 8 + file;
                if let Some(piece) = self.board[sq] {
                    if empty > 0 { fen.push((b'0' + empty) as char); empty = 0; }
                    let ch = match (piece.kind, piece.color) {
                        (PieceType::Pawn,   Color::White) => 'P',
                        (PieceType::Knight, Color::White) => 'N',
                        (PieceType::Bishop, Color::White) => 'B',
                        (PieceType::Rook,   Color::White) => 'R',
                        (PieceType::Queen,  Color::White) => 'Q',
                        (PieceType::King,   Color::White) => 'K',
                        (PieceType::Pawn,   Color::Black) => 'p',
                        (PieceType::Knight, Color::Black) => 'n',
                        (PieceType::Bishop, Color::Black) => 'b',
                        (PieceType::Rook,   Color::Black) => 'r',
                        (PieceType::Queen,  Color::Black) => 'q',
                        (PieceType::King,   Color::Black) => 'k',
                    };
                    fen.push(ch);
                } else {
                    empty += 1;
                }
            }
            if empty > 0 { fen.push((b'0' + empty) as char); }
            if rank > 0 { fen.push('/'); }
        }
        fen.push(' ');
        fen.push(if self.side == Color::White { 'w' } else { 'b' });
        fen.push(' ');
        if self.castling == 0 {
            fen.push('-');
        } else {
            if self.castling & CR_WK != 0 { fen.push('K'); }
            if self.castling & CR_WQ != 0 { fen.push('Q'); }
            if self.castling & CR_BK != 0 { fen.push('k'); }
            if self.castling & CR_BQ != 0 { fen.push('q'); }
        }
        fen.push(' ');
        if self.ep_sq == 255 {
            fen.push('-');
        } else {
            fen.push_str(&sq_to_str(self.ep_sq));
        }
        fen.push_str(&format!(" {} {}", self.halfmove_clock, self.fullmove));
        fen
    }

    pub fn all_occupancy(&self) -> Bitboard {
        self.occupancy[0] | self.occupancy[1]
    }
}
