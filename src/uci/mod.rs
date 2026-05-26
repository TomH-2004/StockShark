use std::io::{self, BufRead, Write};
use std::sync::atomic::Ordering;

use crate::board::position::{Color, Position, PieceType, Move, str_to_sq};
use crate::engine::Engine;
use crate::moves::generator::MoveGen;
use crate::moves::make_move::make_move;

pub fn run_uci() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut pos = Position::startpos();
    let mut engine = Engine::new();
    // Hashes of all positions reached in the current game, for repetition detection
    let mut game_history: Vec<u64> = vec![pos.hash];

    for line in stdin.lock().lines() {
        let line = line.unwrap_or_default();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        match tokens[0] {
            "uci" => {
                writeln!(out, "id name StockShark").unwrap();
                writeln!(out, "id author StockShark Team").unwrap();
                writeln!(out, "uciok").unwrap();
            }
            "isready" => {
                writeln!(out, "readyok").unwrap();
            }
            "ucinewgame" => {
                pos = Position::startpos();
                game_history = vec![pos.hash];
            }
            "position" => {
                (pos, game_history) = parse_position(&tokens);
            }
            "go" => {
                let (depth, time_ms) = parse_go(&tokens, pos.side);
                engine.game_history = game_history.clone();
                let result = engine.search(&mut pos, depth, time_ms);
                writeln!(out, "bestmove {}", result.best_move.to_uci()).unwrap();
            }
            "stop" => {
                engine.stop.store(true, Ordering::Relaxed);
            }
            "quit" => break,
            "d" => {
                print_board(&pos);
            }
            _ => {}
        }
        out.flush().unwrap();
    }
}

/// Returns the position after applying all moves, plus a history of all position hashes.
fn parse_position(tokens: &[&str]) -> (Position, Vec<u64>) {
    let mut pos = if tokens.get(1) == Some(&"startpos") {
        Position::startpos()
    } else if tokens.get(1) == Some(&"fen") {
        let fen_parts: Vec<&str> = tokens[2..].iter()
            .take_while(|&&t| t != "moves")
            .cloned()
            .collect();
        Position::from_fen(&fen_parts.join(" ")).unwrap_or_else(Position::startpos)
    } else {
        Position::startpos()
    };

    let mut history = vec![pos.hash];

    if let Some(moves_idx) = tokens.iter().position(|&t| t == "moves") {
        for mv_str in &tokens[moves_idx + 1..] {
            if let Some(mv) = parse_uci_move(&pos, mv_str) {
                make_move(&mut pos, mv);
                history.push(pos.hash);
            }
        }
    }

    (pos, history)
}

fn parse_uci_move(pos: &Position, s: &str) -> Option<Move> {
    if s.len() < 4 { return None; }
    let from = str_to_sq(&s[0..2])?;
    let to   = str_to_sq(&s[2..4])?;

    let moves = MoveGen::generate(pos);
    moves.into_iter().find(|mv| mv.from_sq() == from && mv.to_sq() == to && {
        if mv.is_promotion() {
            let promo_char = s.chars().nth(4).unwrap_or('q');
            let expected = match mv.promo_piece() {
                PieceType::Queen  => 'q',
                PieceType::Rook   => 'r',
                PieceType::Bishop => 'b',
                PieceType::Knight => 'n',
                _ => 'q',
            };
            promo_char == expected
        } else {
            true
        }
    })
}

fn parse_go(tokens: &[&str], side: Color) -> (u32, Option<u64>) {
    let mut depth = 6u32;
    let mut movetime: Option<u64> = None;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;

    let mut i = 1;
    while i < tokens.len() {
        match tokens[i] {
            "depth"    => { i += 1; depth = tokens.get(i).and_then(|t| t.parse().ok()).unwrap_or(6); }
            "movetime" => { i += 1; movetime = tokens.get(i).and_then(|t| t.parse().ok()); }
            "wtime"    => { i += 1; wtime = tokens.get(i).and_then(|t| t.parse().ok()); }
            "btime"    => { i += 1; btime = tokens.get(i).and_then(|t| t.parse().ok()); }
            _ => {}
        }
        i += 1;
    }

    let time_ms = movetime.or_else(|| {
        let t = match side {
            Color::White => wtime,
            Color::Black => btime,
        };
        t.map(|ms| ms / 30)
    });

    (depth, time_ms)
}

fn print_board(pos: &Position) {
    println!("  +---+---+---+---+---+---+---+---+");
    for rank in (0..8).rev() {
        print!("{} |", rank + 1);
        for file in 0..8 {
            let sq = rank * 8 + file;
            let ch = match pos.board[sq] {
                None => '.',
                Some(p) => match (p.kind, p.color) {
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
                },
            };
            print!(" {} |", ch);
        }
        println!();
        println!("  +---+---+---+---+---+---+---+---+");
    }
    println!("    a   b   c   d   e   f   g   h");
    println!("FEN: {}", pos.to_fen());
}
