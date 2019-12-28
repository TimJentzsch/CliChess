use colored::*;
use pleco::{BitMove, Board, MoveList, Piece, Player, SQ};

const HEIGHT: u8 = 10;
const HISTORY_START: u8 = 2;

pub enum DrawType {
    Stalemate,
    Rule50,
}

pub enum BoardState {
    Win(Player),
    Draw(DrawType),
    Turn(Player),
}

pub enum CliSquareColor {
    White,
    Black,
}

pub struct CliSquare {
    rank: u8,
    row: u8,
    color: CliSquareColor,
}

pub struct CliMove {
    src: SQ,
    dest: SQ,
    piece: Piece,
    player: Player,
    capture: Option<Piece>,
    capture_sq: Option<SQ>,
    promo: Option<Piece>,
    check_sq: Option<SQ>,
}

impl CliMove {
    pub fn new(mv: BitMove, board: Board) -> CliMove {
        let src = mv.get_src();
        let dest = mv.get_dest();
        let piece = board.piece_at_sq(src);
        let player = board.turn();
        let check_sq = if board.gives_check(mv) {
            match player {
                Player::White => Some(board.king_sq(Player::Black)),
                Player::Black => Some(board.king_sq(Player::White)),
            }
        } else {
            None
        };
        let capture_sq = if mv.is_capture() {
            if mv.is_en_passant() {
                match player {
                    Player::White => {
                        let SQ(sq) = dest;
                        Option::Some(SQ(sq - 8))
                    }
                    Player::Black => {
                        let SQ(sq) = dest;
                        Option::Some(SQ(sq + 8))
                    }
                }
            } else {
                Option::Some(dest)
            }
        } else {
            Option::None
        };
        let capture = match capture_sq {
            Option::Some(sq) => Option::Some(board.piece_at_sq(sq)),
            Option::None => Option::None,
        };

        let promo = if mv.is_promo() {
            Option::Some(Piece::make_lossy(player, mv.promo_piece()))
        } else {
            Option::None
        };

        CliMove {
            src: src,
            dest: dest,
            piece: piece,
            player: player,
            capture_sq: capture_sq,
            capture: capture,
            promo: promo,
            check_sq: check_sq,
        }
    }

    pub fn color_str(&self) -> String {
        // Add default info
        let piece_str = CliMove::piece_str(self.piece);
        let src_str = self.src.to_string();
        let dest_str = self.dest.to_string();
        let mut s = format!("{} {} -> {}", piece_str, src_str, dest_str);
        match self.promo {
            Option::None => (),
            Option::Some(promo) => s = format!("{} {}", s, CliMove::piece_str(promo)),
        };
        // Add capture info if applicable
        match self.capture {
            Option::None => (),
            Option::Some(capture) => {
                let cap_str = format! {"{} ", capture.character_lossy()};
                let cap_str = match capture.player_lossy() {
                    Player::White => cap_str.black().on_bright_red().to_string(),
                    Player::Black => cap_str.white().on_red().to_string(),
                };
                s = format!("{} {}", s, cap_str);
            }
        }
        // Add check info if applicable
        match self.check_sq {
            Option::None => (),
            Option::Some(_) => {
                let king_str = match self.player {
                    // White player moved, black player in check
                    Player::White => "k ".black().on_yellow(),
                    // Black player moved, white player in check
                    Player::Black => "K ".black().on_bright_yellow(),
                };
                s = format!("{} {}", s, king_str);
            }
        }
        s
    }

    pub fn piece_str(piece: Piece) -> String {
        let piece_str = piece.character_lossy().to_string() + " ";
        match piece.player_lossy() {
            Player::White => piece_str.black().on_white().to_string(),
            Player::Black => piece_str.white().on_black().to_string(),
        }
    }
}

pub struct CliBoard {
    board: Board,          // The board to display
    history: Vec<CliMove>, // The moves played so far
}

impl CliBoard {
    pub fn new(board: Board) -> CliBoard {
        CliBoard {
            board: board,
            history: Vec::new(),
        }
    }

    pub fn from_fen(fen_str: &str) -> Result<CliBoard, &str> {
        if let Ok(board) = Board::from_fen(fen_str) {
            Result::Ok(CliBoard::new(board))
        } else {
            Result::Err("Invalid fen string!")
        }
    }

    pub fn apply_uci_move(&mut self, uci_move: &str) -> bool {
        let board = self.board.clone();
        let result = self.board.apply_uci_move(uci_move);
        if result {
            let cli_mv = CliMove::new(self.board.last_move().unwrap(), board);
            self.history.push(cli_mv);
            result
        } else {
            result
        }
    }

    pub fn turn(&self) -> Player {
        self.board.turn()
    }

    pub fn board(&self) -> Board {
        self.board.clone()
    }

    pub fn apply_move(&mut self, bit_move: BitMove) {
        let board = self.board.clone();
        self.board.apply_move(bit_move);
        let cli_mv = CliMove::new(bit_move, board);
        self.history.push(cli_mv);
    }

    pub fn generate_moves(&self) -> MoveList {
        self.board.generate_moves()
    }

    pub fn color_print(&self) {
        println!("{}", self.color_string());
    }

    pub fn color_string(&self) -> String {
        let mut s = format!("{}  {}\n", CliBoard::file_header(), self.board_state_str());
        for rev_rank in 0..8 {
            let rank = 8 - rev_rank;
            s += &format!(
                "{} {} {}  {}\n",
                rank,
                self.color_rank_string(rank),
                rank,
                self.history_str(rank)
            );
        }
        s += &CliBoard::file_header();
        s
    }

    fn board_state_str(&self) -> String {
        let rule_50 = self.board.rule_50();
        match self.board_state() {
            BoardState::Win(Player::White) => String::from("White won!"),
            BoardState::Win(Player::Black) => String::from("Black won!"),
            BoardState::Turn(Player::White) => format!("White to move. ({}/50)", rule_50),
            BoardState::Turn(Player::Black) => format!("Black to move. ({}/50)", rule_50),
            BoardState::Draw(DrawType::Stalemate) => String::from("It's a draw (stalemate)."),
            BoardState::Draw(DrawType::Rule50) => String::from("It's a draw (rule 50)."),
        }
    }

    pub fn board_state(&self) -> BoardState {
        if self.board.rule_50() >= 50 {
            BoardState::Draw(DrawType::Rule50)
        } else if self.board.stalemate() {
            BoardState::Draw(DrawType::Stalemate)
        } else if self.board().checkmate() {
            if self.turn() == Player::White {
                BoardState::Win(Player::Black)
            } else {
                BoardState::Win(Player::White)
            }
        } else {
            if self.turn() == Player::White {
                BoardState::Turn(Player::White)
            } else {
                BoardState::Turn(Player::Black)
            }
        }
    }

    fn history_str(&self, row: u8) -> String {
        let rev_index = (HEIGHT - row - HISTORY_START) as usize;
        if rev_index >= self.history.len() {
            String::new()
        } else {
            let index = self.history.len() - rev_index - 1;
            format!("{:03}: {}", index + 1, self.history[index].color_str())
        }
    }

    fn file_header() -> String {
        let s = String::from("  a b c d e f g h  ");
        s
    }

    fn color_rank_string(&self, rank: u8) -> String {
        if rank < 1 || rank > 8 {
            panic!("Rank out of bounds");
        }

        let mut s = String::new();
        for file in 1..9 {
            s += &self.color_square_string(rank, file);
        }
        s
    }

    fn color_square_string(&self, rank: u8, file: u8) -> String {
        if rank < 1 || rank > 8 {
            panic!("Rank out of bounds");
        }
        if file < 1 || file > 8 {
            panic!("File out of bounds");
        }

        let square = SQ((rank - 1) * 8 + file - 1);
        let last_mv = self.history.last();
        let is_capture = match last_mv {
            None => false,
            Some(mv) => {
                if let Some(cap_sq) = mv.capture_sq {
                    cap_sq == square
                } else {
                    false
                }
            }
        };
        let has_changed = match last_mv {
            None => false,
            Some(mv) => {
                let src = mv.src;
                let dest = mv.dest;
                src == square || dest == square
            }
        };

        let piece = self.board.get_piece_locations().piece_at(square);

        let piece_str = if piece != Piece::None {
            piece.character_lossy().to_string() + " "
        } else {
            String::from("  ")
        };

        let is_white = square.on_light_square();

        let is_in_check = if let Some(mv) = last_mv {
            if let Some(sq) = mv.check_sq {
                sq == square
            } else {
                false
            }
        } else {
            false
        };

        let color_string = if is_white {
            if is_in_check {
                piece_str.black().on_bright_yellow()
            } else if is_capture {
                piece_str.black().on_bright_red()
            } else if has_changed {
                piece_str.black().on_bright_blue()
            } else {
                piece_str.black().on_white()
            }
        } else {
            if is_in_check {
                piece_str.black().on_yellow()
            } else if is_capture {
                piece_str.white().on_red()
            } else if has_changed {
                piece_str.white().on_blue()
            } else {
                piece_str.white().on_black()
            }
        };

        color_string.to_string()
    }
}
