/// A 64-bit integer representing occupancy of squares on the board.
/// Bit 0 = a1, bit 7 = h1, bit 56 = a8, bit 63 = h8.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(u64::MAX);

    pub const FILE_A: Self = Self(0x0101010101010101);
    pub const FILE_H: Self = Self(0x8080808080808080);
    pub const RANK_1: Self = Self(0x00000000000000FF);
    pub const RANK_8: Self = Self(0xFF00000000000000);

    #[inline]
    pub fn from_square(sq: u8) -> Self {
        Self(1u64 << sq)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn contains(self, sq: u8) -> bool {
        (self.0 >> sq) & 1 == 1
    }

    #[inline]
    pub fn set(&mut self, sq: u8) {
        self.0 |= 1u64 << sq;
    }

    #[inline]
    pub fn clear(&mut self, sq: u8) {
        self.0 &= !(1u64 << sq);
    }

    #[inline]
    pub fn pop_lsb(&mut self) -> u8 {
        let sq = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        sq
    }

    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    #[inline]
    pub fn shift_north(self) -> Self {
        Self(self.0 << 8)
    }

    #[inline]
    pub fn shift_south(self) -> Self {
        Self(self.0 >> 8)
    }

    #[inline]
    pub fn shift_east(self) -> Self {
        Self((self.0 << 1) & !Self::FILE_A.0)
    }

    #[inline]
    pub fn shift_west(self) -> Self {
        Self((self.0 >> 1) & !Self::FILE_H.0)
    }
}

impl std::ops::BitOr for Bitboard {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}
impl std::ops::BitAnd for Bitboard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
}
impl std::ops::Not for Bitboard {
    type Output = Self;
    fn not(self) -> Self { Self(!self.0) }
}
impl std::ops::BitXor for Bitboard {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self { Self(self.0 ^ rhs.0) }
}
impl std::ops::BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}
impl std::ops::BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
}

impl Iterator for Bitboard {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.is_empty() { None } else { Some(self.pop_lsb()) }
    }
}
