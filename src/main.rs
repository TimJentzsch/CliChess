mod cli_board;
mod chess_player;
mod mcts;

use pleco::*;
use cli_board::{CliBoard, BoardState};
use chess_player::{HumanPlayer, ChessPlayer, RandomPlayer, StoneFish};
use std::env;

use std::time::{Duration, SystemTime};


fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let board = Board::start_pos();
    let mut cli_board = CliBoard::new(board);
    // let en_passent_fen = "4k3/pppppppp/8/3P4/8/8/8/RNBQKBNR b KQkq - 0 1";
    // let mut cli_board = CliBoard::from_fen(en_passent_fen).unwrap();

    let mut white_player = HumanPlayer::new();
    let mut black_player = StoneFish::new(Player::Black, &cli_board.board());
    // let mut black_player = HumanPlayer::new();

    let mut time = Duration::from_secs(10);
    let min_time = Duration::from_secs(10);
    let max_time = Duration::from_secs(300);

    loop {
        cli_board.color_print();
        let board = cli_board.board();
        let before = SystemTime::now();

        match cli_board.board_state() {
            BoardState::Turn(player) => {
                let bit_move = if player == Player::White {
                    white_player.next_move(&board, time)
                } else {
                    black_player.next_move(&board, time)
                };
        
                cli_board.apply_move(bit_move);
            },
            BoardState::Win(_) => break,
            BoardState::Draw(_) => break,
        }

        let new_time = before.elapsed().unwrap();
        
        time = if new_time < min_time {
            min_time
        } else if new_time > max_time {
            max_time
        } else {
            new_time
        }
    }
}