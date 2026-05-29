use std::sync::{Arc, Mutex};
use std::thread;

use egui::{Color32, Context, Key, Pos2, Rect, Sense, Stroke, Vec2};

use crate::board::position::{Color, Move, Piece, PieceType, Position};
use crate::engine::book::get_book_move;
use crate::engine::eval::is_insufficient_material;
use crate::engine::polyglot::PolyglotBook;
use crate::engine::{Engine, SearchResult};
use crate::moves::generator::MoveGen;
use crate::moves::make_move::make_move;

const SQUARE_SIZE: f32 = 80.0;
const BOARD_SIZE: f32  = SQUARE_SIZE * 8.0;

const LIGHT_SQ:   Color32 = Color32::from_rgb(240, 217, 181);
const DARK_SQ:    Color32 = Color32::from_rgb(181, 136, 99);
const SEL_SQ:     Color32 = Color32::from_rgb(106, 168, 79);
const HINT_SQ:    Color32 = Color32::from_rgb(106, 168, 79);
const LAST_SQ:    Color32 = Color32::from_rgb(205, 210, 106);
const REVIEW_SQ:  Color32 = Color32::from_rgb(100, 149, 237); // cornflower blue tint for review mode

#[derive(Clone, PartialEq)]
pub enum GameMode {
    HumanVsEngine,
    EngineVsHuman,
    HumanVsHuman,
    EngineVsEngine,
}

pub struct StockSharkApp {
    /// The live game position — always the latest state regardless of what we're viewing.
    pos: Position,
    /// Full history: snapshots[0] = start, snapshots[n] = position after n moves.
    position_snapshots: Vec<Position>,
    /// Which snapshot is currently displayed (may be behind the live position in review mode).
    view_index: usize,
    selected_sq: Option<u8>,
    legal_moves: Vec<Move>,
    move_history: Vec<(Move, String)>,
    game_mode: GameMode,
    engine_depth: u32,
    status: String,
    engine_thinking: bool,
    engine_result: Arc<Mutex<Option<SearchResult>>>,
    _human_color: Color,
    eval_score: Option<i32>,
    flip_board: bool,
    /// Per-game seed for opening book variety.
    game_seed: u64,
    /// Loaded Polyglot .bin book, if any.
    polyglot_book: Option<PolyglotBook>,
    /// Path shown in the UI for book loading.
    book_path_input: String,
    book_status: String,
}

impl StockSharkApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let pos = Position::startpos();
        let snapshots = vec![pos.clone()];
        let seed = time_seed();

        // Auto-load: try common names in order.
        let candidates = ["Titans.bin", "titans.bin", "book.bin"];
        let (default_path, polyglot_book, book_status) = candidates.iter()
            .find_map(|&name| {
                PolyglotBook::load(name).map(|b| (
                    name.to_string(),
                    Some(b),
                    format!("Loaded: {}", name),
                ))
            })
            .unwrap_or_else(|| (
                "Titans.bin".to_string(),
                None,
                "No book found — drop Titans.bin here or use Load".to_string(),
            ));

        Self {
            pos,
            position_snapshots: snapshots,
            view_index: 0,
            selected_sq: None,
            legal_moves: Vec::new(),
            move_history: Vec::new(),
            game_mode: GameMode::HumanVsEngine,
            engine_depth: 6,
            status: "Your move (White)".to_string(),
            engine_thinking: false,
            engine_result: Arc::new(Mutex::new(None)),
            _human_color: Color::White,
            eval_score: None,
            flip_board: false,
            game_seed: seed,
            polyglot_book,
            book_path_input: default_path,
            book_status,
        }
    }

    // ── Navigation ──────────────────────────────────────────────────────────

    fn is_reviewing(&self) -> bool {
        self.view_index < self.position_snapshots.len() - 1
    }

    fn viewed_pos(&self) -> &Position {
        &self.position_snapshots[self.view_index]
    }

    fn navigate_to(&mut self, idx: usize) {
        let max = self.position_snapshots.len() - 1;
        self.view_index = idx.min(max);
        self.selected_sq = None;
        self.legal_moves.clear();
    }

    fn nav_first(&mut self) { self.navigate_to(0); }
    fn nav_prev(&mut self)  { if self.view_index > 0 { self.navigate_to(self.view_index - 1); } }
    fn nav_next(&mut self)  { self.navigate_to(self.view_index + 1); }
    fn nav_last(&mut self)  { let last = self.position_snapshots.len() - 1; self.navigate_to(last); }

    // ── Move handling ────────────────────────────────────────────────────────

    fn square_at(&self, ptr: Pos2, board_origin: Pos2) -> Option<u8> {
        let rel = ptr - board_origin;
        if rel.x < 0.0 || rel.y < 0.0 || rel.x >= BOARD_SIZE || rel.y >= BOARD_SIZE {
            return None;
        }
        let file = (rel.x / SQUARE_SIZE) as u8;
        let rank = 7 - (rel.y / SQUARE_SIZE) as u8;
        let (file, rank) = if self.flip_board { (7 - file, 7 - rank) } else { (file, rank) };
        Some(rank * 8 + file)
    }

    fn square_rect(&self, sq: u8, board_origin: Pos2) -> Rect {
        let file = sq % 8;
        let rank = sq / 8;
        let (df, dr) = if self.flip_board { (7 - file, rank) } else { (file, 7 - rank) };
        let x = board_origin.x + df as f32 * SQUARE_SIZE;
        let y = board_origin.y + dr as f32 * SQUARE_SIZE;
        Rect::from_min_size(Pos2::new(x, y), Vec2::splat(SQUARE_SIZE))
    }

    fn handle_click(&mut self, sq: u8) {
        // Board clicks do nothing in review mode
        if self.is_reviewing() { return; }

        let is_human_turn = match self.game_mode {
            GameMode::HumanVsEngine  => self.pos.side == Color::White,
            GameMode::EngineVsHuman  => self.pos.side == Color::Black,
            GameMode::HumanVsHuman   => true,
            GameMode::EngineVsEngine => false,
        };
        if !is_human_turn || self.engine_thinking { return; }

        if let Some(sel) = self.selected_sq {
            if let Some(mv) = self.legal_moves.iter().find(|m| m.from_sq() == sel && m.to_sq() == sq).cloned() {
                self.apply_move(mv);
                self.selected_sq = None;
                self.legal_moves.clear();
                return;
            }
        }

        if let Some(piece) = self.pos.board[sq as usize] {
            if piece.color == self.pos.side {
                self.selected_sq = Some(sq);
                self.legal_moves = MoveGen::generate(&self.pos)
                    .into_iter()
                    .filter(|mv| mv.from_sq() == sq && self.is_legal(mv))
                    .collect();
                return;
            }
        }

        self.selected_sq = None;
        self.legal_moves.clear();
    }

    fn is_legal(&self, mv: &Move) -> bool {
        let mut pos = self.pos.clone();
        let _undo = make_move(&mut pos, *mv);
        let us = self.pos.side as usize;
        !MoveGen::is_attacked(&pos, pos.king_sq[us], 1 - us)
    }

    fn apply_move(&mut self, mv: Move) {
        let notation = mv.to_uci();
        make_move(&mut self.pos, mv);
        self.position_snapshots.push(self.pos.clone());
        self.view_index = self.position_snapshots.len() - 1;
        self.move_history.push((mv, notation));
        self.update_status();

        if !self.is_game_over() {
            self.maybe_trigger_engine();
        }
    }

    fn maybe_trigger_engine(&mut self) {
        let engine_turn = match self.game_mode {
            GameMode::HumanVsEngine  => self.pos.side == Color::Black,
            GameMode::EngineVsHuman  => self.pos.side == Color::White,
            GameMode::EngineVsEngine => true,
            GameMode::HumanVsHuman   => false,
        };
        if engine_turn && !self.engine_thinking && !self.try_book_move() {
            self.start_engine_search();
        }
    }

    /// Check the opening book and, if a move is available, queue it as an
    /// instant engine result. Returns `true` when a book move was queued.
    /// Priority: Polyglot file → internal hardcoded lines.
    fn try_book_move(&mut self) -> bool {
        // 1. Polyglot book (weight-proportional random selection for variety).
        let poly_uci = if let Some(ref book) = self.polyglot_book {
            let entries = book.probe(&self.pos);
            if !entries.is_empty() {
                // Pick randomly weighted by entry weight using the game seed.
                let total: u32 = entries.iter().map(|e| e.weight as u32).sum();
                let mut pick = (xorshift64(self.game_seed.wrapping_add(self.move_history.len() as u64))
                    % total as u64) as u32;
                let mut chosen = entries[0].uci.clone();
                for e in &entries {
                    if pick < e.weight as u32 { chosen = e.uci.clone(); break; }
                    pick -= e.weight as u32;
                }
                Some(chosen)
            } else {
                None
            }
        } else {
            None
        };

        // 2. Fallback to internal hardcoded lines.
        let uci_history: Vec<String> = self.move_history.iter().map(|(_, s)| s.clone()).collect();
        let book_uci = poly_uci
            .or_else(|| get_book_move(&uci_history, self.game_seed));

        if let Some(uci) = book_uci {
            if let Some(mv) = self.find_legal_move_by_uci(&uci) {
                self.engine_thinking = true;
                self.status = "Book...".to_string();
                *self.engine_result.lock().unwrap() = Some(SearchResult {
                    best_move: mv,
                    score: 0,
                    depth: 0,
                    nodes: 0,
                    pv: vec![mv],
                });
                return true;
            }
        }
        false
    }

    fn find_legal_move_by_uci(&self, uci: &str) -> Option<Move> {
        let moves = crate::moves::generator::MoveGen::generate(&self.pos);
        moves.into_iter().find(|mv| mv.to_uci() == uci && self.is_legal(mv))
    }

    fn start_engine_search(&mut self) {
        self.engine_thinking = true;
        self.status = "Engine thinking...".to_string();

        let mut pos = self.pos.clone();
        let depth = self.engine_depth;
        let result_ref = self.engine_result.clone();
        // Exclude the current position so the repetition check in negamax doesn't
        // false-trigger at the root and return a null move (causing a freeze).
        let n = self.position_snapshots.len();
        let history: Vec<u64> = self.position_snapshots[..n.saturating_sub(1)]
            .iter()
            .map(|p| p.hash)
            .collect();

        thread::spawn(move || {
            let mut engine = Engine::new();
            engine.game_history = history;
            let result = engine.search(&mut pos, depth, Some(3000));
            *result_ref.lock().unwrap() = Some(result);
        });
    }

    fn poll_engine(&mut self) {
        if !self.engine_thinking { return; }
        let result = self.engine_result.lock().unwrap().take();
        if let Some(r) = result {
            self.eval_score = Some(r.score);
            self.engine_thinking = false;
            if !r.best_move.is_null() {
                self.apply_move(r.best_move);
            } else {
                self.update_status();
            }
        }
    }

    fn update_status(&mut self) {
        if self.is_game_over() { return; }
        let side = if self.pos.side == Color::White { "White" } else { "Black" };
        if MoveGen::in_check(&self.pos) {
            self.status = format!("{} is in check!", side);
        } else {
            self.status = format!("{}'s turn", side);
        }
    }

    fn is_game_over(&mut self) -> bool {
        if is_insufficient_material(&self.pos) {
            self.status = "Draw by insufficient material.".to_string();
            return true;
        }
        let moves = MoveGen::generate(&self.pos);
        let has_legal = moves.iter().any(|mv| self.is_legal(mv));
        if !has_legal {
            if MoveGen::in_check(&self.pos) {
                let winner = if self.pos.side == Color::White { "Black" } else { "White" };
                self.status = format!("Checkmate! {} wins.", winner);
            } else {
                self.status = "Stalemate! Draw.".to_string();
            }
            true
        } else if self.pos.halfmove_clock >= 100 {
            self.status = "Draw by 50-move rule.".to_string();
            true
        } else {
            false
        }
    }

    fn new_game(&mut self) {
        self.pos = Position::startpos();
        self.position_snapshots = vec![self.pos.clone()];
        self.view_index = 0;
        self.selected_sq = None;
        self.legal_moves.clear();
        self.move_history.clear();
        self.engine_thinking = false;
        self.eval_score = None;
        self.game_seed = time_seed();
        *self.engine_result.lock().unwrap() = None;
        self.update_status();
        self.maybe_trigger_engine();
    }

    // ── Drawing ──────────────────────────────────────────────────────────────

    fn last_move_squares(&self) -> Option<(u8, u8)> {
        if self.view_index == 0 { return None; }
        let mv = self.move_history.get(self.view_index - 1)?.0;
        Some((mv.from_sq(), mv.to_sq()))
    }

    fn draw_board(&self, painter: &egui::Painter, origin: Pos2) {
        let reviewing = self.is_reviewing();
        for sq in 0..64u8 {
            let rect = self.square_rect(sq, origin);
            let file = sq % 8;
            let rank = sq / 8;
            let is_light = (file + rank) % 2 == 0;

            let mut color = if is_light { LIGHT_SQ } else { DARK_SQ };

            // Highlight the move that led to the viewed position
            if let Some((from, to)) = self.last_move_squares() {
                if sq == from || sq == to {
                    color = if reviewing { REVIEW_SQ.gamma_multiply(0.6) } else { LAST_SQ };
                }
            }
            if self.selected_sq == Some(sq) {
                color = SEL_SQ;
            }

            // Dim non-reviewed squares slightly to signal review mode
            let color = if reviewing { color.gamma_multiply(0.88) } else { color };
            painter.rect_filled(rect, 0.0, color);

            // Rank/file labels
            if file == 0 {
                painter.text(
                    rect.min + Vec2::new(2.0, 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", rank + 1),
                    egui::FontId::proportional(12.0),
                    if is_light { DARK_SQ } else { LIGHT_SQ },
                );
            }
            if rank == 0 {
                painter.text(
                    rect.max - Vec2::new(6.0, 2.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{}", (b'a' + file) as char),
                    egui::FontId::proportional(12.0),
                    if is_light { DARK_SQ } else { LIGHT_SQ },
                );
            }

            // Legal move hints (only shown when at the live position)
            if !reviewing {
                if let Some(hint_mv) = self.legal_moves.iter().find(|m| m.to_sq() == sq) {
                    if hint_mv.is_capture() {
                        painter.circle_stroke(rect.center(), SQUARE_SIZE * 0.45, Stroke::new(4.0, HINT_SQ.gamma_multiply(0.7)));
                    } else {
                        painter.circle_filled(rect.center(), SQUARE_SIZE * 0.15, HINT_SQ.gamma_multiply(0.7));
                    }
                }
            }
        }
    }

    fn draw_pieces(&self, painter: &egui::Painter, origin: Pos2) {
        let viewed = self.viewed_pos();
        for sq in 0..64u8 {
            if let Some(piece) = viewed.board[sq as usize] {
                let rect = self.square_rect(sq, origin);
                let glyph = piece_glyph(piece);
                let (text_color, stroke_color) = if piece.color == Color::White {
                    (Color32::WHITE, Color32::from_gray(30))
                } else {
                    (Color32::from_gray(20), Color32::from_gray(180))
                };
                painter.text(
                    rect.center() + Vec2::new(1.5, 1.5),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(56.0),
                    stroke_color.gamma_multiply(0.5),
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(56.0),
                    text_color,
                );
            }
        }
    }

    fn draw_eval_bar(&self, ui: &mut egui::Ui) {
        let score = self.eval_score.unwrap_or(0) as f32;
        let clamped = score.clamp(-1000.0, 1000.0);
        let white_frac = (clamped / 1000.0) * 0.5 + 0.5;

        let (_, rect) = ui.allocate_space(Vec2::new(20.0, BOARD_SIZE));
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));
        let white_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.max.y - rect.height() * white_frac),
            Vec2::new(rect.width(), rect.height() * white_frac),
        );
        painter.rect_filled(white_rect, 0.0, Color32::WHITE);

        let score_str = if score.abs() > 0.5 {
            format!("{:+.1}", score / 100.0)
        } else {
            "0.0".to_string()
        };
        painter.text(
            Pos2::new(rect.center().x, rect.min.y + 4.0),
            egui::Align2::CENTER_TOP,
            &score_str,
            egui::FontId::proportional(11.0),
            Color32::DARK_GRAY,
        );
    }
}

impl eframe::App for StockSharkApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_engine();

        if self.engine_thinking {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Keyboard navigation (left/right arrows)
        ctx.input(|i| {
            if i.key_pressed(Key::ArrowLeft)  { /* handled below via borrow split */ }
            if i.key_pressed(Key::ArrowRight) { /* handled below via borrow split */ }
        });
        let go_prev = ctx.input(|i| i.key_pressed(Key::ArrowLeft));
        let go_next = ctx.input(|i| i.key_pressed(Key::ArrowRight));
        if go_prev { self.nav_prev(); }
        if go_next { self.nav_next(); }

        // ── Top toolbar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("StockShark");
                ui.separator();
                if ui.button("New Game").clicked() { self.new_game(); }
                ui.separator();
                ui.label("Mode:");
                egui::ComboBox::from_id_salt("mode")
                    .selected_text(match self.game_mode {
                        GameMode::HumanVsEngine  => "Human vs Engine",
                        GameMode::EngineVsHuman  => "Engine vs Human",
                        GameMode::HumanVsHuman   => "Human vs Human",
                        GameMode::EngineVsEngine => "Engine vs Engine",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.game_mode, GameMode::HumanVsEngine,  "Human vs Engine");
                        ui.selectable_value(&mut self.game_mode, GameMode::EngineVsHuman,  "Engine vs Human");
                        ui.selectable_value(&mut self.game_mode, GameMode::HumanVsHuman,   "Human vs Human");
                        ui.selectable_value(&mut self.game_mode, GameMode::EngineVsEngine, "Engine vs Engine");
                    });
                ui.separator();
                ui.label("Depth:");
                ui.add(egui::Slider::new(&mut self.engine_depth, 1..=12));
                ui.separator();
                if ui.button(if self.flip_board { "Flip (on)" } else { "Flip board" }).clicked() {
                    self.flip_board = !self.flip_board;
                }
            });
            // ── Opening book loader ──────────────────────────────────────────
            ui.horizontal(|ui| {
                let book_indicator = if self.polyglot_book.is_some() { "📖" } else { "—" };
                ui.label(format!("Book {}", book_indicator));
                ui.add(egui::TextEdit::singleline(&mut self.book_path_input).desired_width(220.0));
                if ui.button("Load").clicked() {
                    let path = self.book_path_input.clone();
                    match PolyglotBook::load(&path) {
                        Some(b) => {
                            self.polyglot_book = Some(b);
                            self.book_status = format!("Loaded: {}", path);
                        }
                        None => {
                            self.book_status = format!("Failed to load: {}", path);
                        }
                    }
                }
                if ui.button("Clear").clicked() {
                    self.polyglot_book = None;
                    self.book_status = "No book loaded".to_string();
                }
                ui.label(egui::RichText::new(&self.book_status)
                    .color(if self.polyglot_book.is_some() {
                        Color32::from_rgb(100, 200, 100)
                    } else {
                        Color32::GRAY
                    }));
            });
        });

        // ── Move list (right panel) ──────────────────────────────────────────
        egui::SidePanel::right("move_list").min_width(170.0).show(ctx, |ui| {
            ui.heading("Moves");
            ui.separator();

            // Navigation row
            ui.horizontal(|ui| {
                let at_start = self.view_index == 0;
                let at_end   = !self.is_reviewing();
                if ui.add_enabled(!at_start, egui::Button::new("|<")).clicked() { self.nav_first(); }
                if ui.add_enabled(!at_start, egui::Button::new(" < ")).clicked() { self.nav_prev(); }
                if ui.add_enabled(!at_end,   egui::Button::new(" > ")).clicked() { self.nav_next(); }
                if ui.add_enabled(!at_end,   egui::Button::new(">|")).clicked() { self.nav_last(); }
                ui.label(format!("{}/{}", self.view_index, self.position_snapshots.len() - 1));
            });

            ui.separator();

            // Scrollable move list; clicking a move jumps to that position
            let current_view = self.view_index;
            let mut jump_to: Option<usize> = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (i, (_, notation)) in self.move_history.iter().enumerate() {
                        // snapshot index that this move leads to = i + 1
                        let snap_idx = i + 1;
                        let is_current = snap_idx == current_view;

                        if i % 2 == 0 {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}.", i / 2 + 1));
                                let btn = egui::Button::new(notation)
                                    .fill(if is_current { REVIEW_SQ.gamma_multiply(0.4) } else { Color32::TRANSPARENT });
                                if ui.add(btn).clicked() {
                                    jump_to = Some(snap_idx);
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.add_space(28.0); // indent for Black's move
                                let btn = egui::Button::new(notation)
                                    .fill(if is_current { REVIEW_SQ.gamma_multiply(0.4) } else { Color32::TRANSPARENT });
                                if ui.add(btn).clicked() {
                                    jump_to = Some(snap_idx);
                                }
                            });
                        }
                    }
                });

            if let Some(idx) = jump_to {
                self.navigate_to(idx);
            }
        });

        // ── Central board panel ──────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Status line — show review indicator when browsing history
                if self.is_reviewing() {
                    let _total = self.position_snapshots.len() - 1;
                    let move_num = (self.view_index + 1) / 2;
                    let side_str = if self.view_index % 2 == 1 { "White" } else { "Black" };
                    ui.colored_label(
                        REVIEW_SQ,
                        format!("Reviewing: move {} ({}) — press \u{2192} or click >| to return", move_num, side_str),
                    );
                } else {
                    ui.label(&self.status);
                }
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    self.draw_eval_bar(ui);
                    ui.add_space(4.0);

                    let (resp, painter) = ui.allocate_painter(Vec2::splat(BOARD_SIZE), Sense::click());
                    let origin = resp.rect.min;

                    self.draw_board(&painter, origin);
                    self.draw_pieces(&painter, origin);

                    if resp.clicked() {
                        if let Some(click_pos) = resp.interact_pointer_pos() {
                            if let Some(sq) = self.square_at(click_pos, origin) {
                                self.handle_click(sq);
                            }
                        }
                    }
                });
            });
        });
    }
}

fn xorshift64(mut x: u64) -> u64 {
    if x == 0 { x = 0x9e3779b97f4a7c15; }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e3779b97f4a7c15)
}

fn piece_glyph(piece: Piece) -> &'static str {
    match (piece.kind, piece.color) {
        (PieceType::King,   Color::White) => "♔",
        (PieceType::Queen,  Color::White) => "♕",
        (PieceType::Rook,   Color::White) => "♖",
        (PieceType::Bishop, Color::White) => "♗",
        (PieceType::Knight, Color::White) => "♘",
        (PieceType::Pawn,   Color::White) => "♙",
        (PieceType::King,   Color::Black) => "♚",
        (PieceType::Queen,  Color::Black) => "♛",
        (PieceType::Rook,   Color::Black) => "♜",
        (PieceType::Bishop, Color::Black) => "♝",
        (PieceType::Knight, Color::Black) => "♞",
        (PieceType::Pawn,   Color::Black) => "♟",
    }
}
