/// Zobrist hashing for position identification and transposition tables.
/// Keys are generated deterministically from a fixed seed.

const fn lcg(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

const fn gen_keys() -> ([[u64; 64]; 12], u64, [u64; 4], u64) {
    let mut pieces = [[0u64; 64]; 12];
    let mut state = 0xDEADBEEFCAFEBABEu64;

    let mut p = 0usize;
    while p < 12 {
        let mut sq = 0usize;
        while sq < 64 {
            state = lcg(state);
            pieces[p][sq] = state;
            sq += 1;
        }
        p += 1;
    }

    state = lcg(state);
    let side = state;

    let mut castling = [0u64; 4];
    let mut i = 0usize;
    while i < 4 {
        state = lcg(state);
        castling[i] = state;
        i += 1;
    }

    state = lcg(state);
    let ep_base = state;

    (pieces, side, castling, ep_base)
}

static KEYS: std::sync::LazyLock<ZobristKeys> = std::sync::LazyLock::new(|| {
    let (pieces, side, castling, ep_base) = gen_keys();
    let mut ep = [0u64; 8];
    let mut s = ep_base;
    for i in 0..8 {
        s = lcg(s);
        ep[i] = s;
    }
    ZobristKeys { pieces, side, castling, ep }
});

struct ZobristKeys {
    pieces: [[u64; 64]; 12],
    side: u64,
    castling: [u64; 4],
    ep: [u64; 8],
}

/// piece index: piece_type * 2 + color (0=white, 1=black), piece_type 0-5
pub fn piece_key(piece_idx: usize, sq: u8) -> u64 {
    KEYS.pieces[piece_idx][sq as usize]
}

pub fn side_key() -> u64 {
    KEYS.side
}

pub fn castling_key(rights: u8) -> u64 {
    let mut h = 0u64;
    for i in 0..4 {
        if rights & (1 << i) != 0 {
            h ^= KEYS.castling[i];
        }
    }
    h
}

pub fn ep_key(file: u8) -> u64 {
    KEYS.ep[file as usize]
}
