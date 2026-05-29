/// Opening book: each entry is a sequence of UCI moves from the starting position.
const LINES: &[&[&str]] = &[
    // ── Ruy Lopez ────────────────────────────────────────────────────────────
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "g8f6"],                          // Berlin
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6", "e1g1"], // Morphy/Closed
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "f8c5", "e1g1"],                  // Classical

    // ── Italian ──────────────────────────────────────────────────────────────
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5", "d2d3"],                  // Giuoco Piano
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6", "d2d3"],                  // Two Knights
    &["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5", "c2c3", "g8f6"],          // Giuoco Pianissimo

    // ── Scotch ───────────────────────────────────────────────────────────────
    &["e2e4", "e7e5", "g1f3", "b8c6", "d2d4", "e5d4", "f3d4"],

    // ── King's Gambit ────────────────────────────────────────────────────────
    &["e2e4", "e7e5", "f2f4", "e5f4", "g1f3"],

    // ── Sicilian – Open ──────────────────────────────────────────────────────
    &["e2e4", "c7c5", "g1f3", "d7d6", "d2d4", "c5d4", "f3d4", "g8f6", "b1c3", "a7a6"],  // Najdorf
    &["e2e4", "c7c5", "g1f3", "d7d6", "d2d4", "c5d4", "f3d4", "g8f6", "b1c3", "g7g6"],  // Dragon
    &["e2e4", "c7c5", "g1f3", "e7e6", "d2d4", "c5d4", "f3d4", "g8f6", "b1c3", "d7d6"],  // Scheveningen
    &["e2e4", "c7c5", "g1f3", "e7e6", "d2d4", "c5d4", "f3d4", "a7a6"],                   // Kan/Taimanov
    &["e2e4", "c7c5", "b1c3", "b8c6", "g2g3", "g7g6", "f1g2"],                           // Closed Sicilian

    // ── French ───────────────────────────────────────────────────────────────
    &["e2e4", "e7e6", "d2d4", "d7d5", "b1c3", "g8f6", "c1g5"],                  // Classical
    &["e2e4", "e7e6", "d2d4", "d7d5", "e4e5", "c7c5", "c2c3"],                  // Advance
    &["e2e4", "e7e6", "d2d4", "d7d5", "b1d2", "g8f6"],                          // Tarrasch

    // ── Caro-Kann ────────────────────────────────────────────────────────────
    &["e2e4", "c7c6", "d2d4", "d7d5", "b1c3", "d5e4", "c3e4", "g8f6"],          // Classical
    &["e2e4", "c7c6", "d2d4", "d7d5", "e4e5", "c8f5"],                          // Advance

    // ── Scandinavian ─────────────────────────────────────────────────────────
    &["e2e4", "d7d5", "e4d5", "d8d5", "b1c3", "d5a5"],

    // ── Pirc / Modern ────────────────────────────────────────────────────────
    &["e2e4", "d7d6", "d2d4", "g8f6", "b1c3", "g7g6", "f1e2"],
    &["e2e4", "g7g6", "d2d4", "f8g7", "b1c3", "d7d6", "g1f3"],                  // Modern

    // ── d4 openings ─────────────────────────────────────────────────────────
    &["d2d4", "d7d5", "c2c4", "e7e6", "b1c3", "g8f6", "c1g5"],                  // QGD Classical
    &["d2d4", "d7d5", "c2c4", "d5c4", "g1f3", "g8f6", "e2e3"],                  // QGA
    &["d2d4", "d7d5", "c2c4", "c7c6", "g1f3", "g8f6", "b1c3", "e7e6"],          // Slav
    &["d2d4", "g8f6", "c2c4", "g7g6", "b1c3", "f8g7", "e2e4", "d7d6", "g1f3"], // King's Indian
    &["d2d4", "g8f6", "c2c4", "e7e6", "b1c3", "f8b4", "e2e3"],                  // Nimzo-Indian
    &["d2d4", "g8f6", "c2c4", "e7e6", "g1f3", "b7b6", "g2g3", "c8b7"],          // Queen's Indian
    &["d2d4", "g8f6", "c2c4", "g7g6", "b1c3", "d7d5", "c4d5", "f6d5"],          // Grünfeld Exchange
    &["d2d4", "f7f5", "g1f3", "g8f6", "g2g3", "e7e6"],                          // Dutch
    &["d2d4", "d7d5", "g1f3", "g8f6", "c1f4", "e7e6", "e2e3", "f8d6"],          // London System
    &["d2d4", "d7d5", "g1f3", "g8f6", "e2e3", "e7e6", "f1d3", "f8d6"],          // Colle System

    // ── English ──────────────────────────────────────────────────────────────
    &["c2c4", "e7e5", "b1c3", "g8f6", "g1f3", "b8c6"],
    &["c2c4", "g8f6", "b1c3", "e7e6", "e2e4", "f8b4"],
    &["c2c4", "c7c5", "g1f3", "g8f6", "b1c3", "b8c6"],

    // ── Réti ─────────────────────────────────────────────────────────────────
    &["g1f3", "d7d5", "g2g3", "g8f6", "f1g2", "e7e6"],
    &["g1f3", "g8f6", "c2c4", "g7g6", "g2g3", "f8g7"],

    // ── Flank / miscellaneous ────────────────────────────────────────────────
    &["b2b3", "e7e5", "c1b2", "b8c6", "g1f3"],
    &["g2g3", "d7d5", "f1g2", "c7c5", "g1f3"],
];

/// Return a book move for the given game move history, or `None` if out of book.
/// `seed` is a per-game constant used to pick among equally-valid book moves so
/// different games follow different lines.
pub fn get_book_move(move_history: &[String], seed: u64) -> Option<String> {
    let n = move_history.len();

    let mut candidates: Vec<&str> = LINES
        .iter()
        .filter(|line| {
            line.len() > n
                && line[..n]
                    .iter()
                    .zip(move_history.iter())
                    .all(|(book_mv, game_mv)| *book_mv == game_mv.as_str())
        })
        .map(|line| line[n])
        .collect();

    candidates.sort_unstable();
    candidates.dedup();

    if candidates.is_empty() {
        return None;
    }

    // Mix seed with depth so deeper plies can pick a different candidate than
    // earlier plies even within the same game, keeping play consistent but varied.
    let idx = xorshift(seed.wrapping_add(n as u64)) as usize % candidates.len();
    Some(candidates[idx].to_string())
}

fn xorshift(mut x: u64) -> u64 {
    if x == 0 {
        x = 0x9e3779b97f4a7c15;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}
