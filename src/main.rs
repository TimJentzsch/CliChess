mod chess_player;
mod cli_board;
mod mcts;

use chess_player::{ChessPlayer, HumanPlayer, RandomPlayer, StoneFish};
use cli_board::{BoardState, CliBoard};
use pleco::*;
use std::env;
use std::sync::{
    mpsc::{self, TryRecvError},
    Arc, Mutex,
};
use std::thread;

use std::time::{Duration, SystemTime};

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let board = Board::start_pos();
    let mut cli_board = CliBoard::new(board);
    // let en_passent_fen = "4k3/pppppppp/8/3P4/8/8/8/RNBQKBNR b KQkq - 0 1";
    // let mut cli_board = CliBoard::from_fen(en_passent_fen).unwrap();

    // let mut white_player = HumanPlayer::new();
    let white_player = RandomPlayer::new();
    let black_player = StoneFish::new(Player::Black, &cli_board.board());

    let white_ref = Arc::new(Mutex::new(white_player));
    let black_ref = Arc::new(Mutex::new(black_player));
    // let mut black_player = HumanPlayer::new();

    let mut time = Duration::from_secs(10);
    let min_time = Duration::from_secs(10);
    let max_time = Duration::from_secs(300);

    loop {
        cli_board.color_print();
        let board = cli_board.board();
        let before = SystemTime::now();
        let ponder_ref = Arc::new(Mutex::new(0));

        match cli_board.board_state() {
            BoardState::Turn(player) => {
                let (tx, rx) = mpsc::channel();
                match player {
                    Player::White => {
                        let th_board = board.clone();
                        let th_black_ref = Arc::clone(&black_ref);
                        let th_ponder_ref = Arc::clone(&ponder_ref);
                        let handle = thread::spawn(move || {
                            // Check if move has been played, else ponder
                            while rx.try_recv().is_err() {
                                let mut black_player = th_black_ref.lock().unwrap();
                                let mut ponder_cnt = th_ponder_ref.lock().unwrap();

                                // Ponder
                                black_player.ponder(&th_board);
                                *ponder_cnt += 1;

                                thread::sleep(Duration::from_millis(5));
                            }
                        });

                        let mut white_player = white_ref.lock().unwrap();
                        let bit_move = (*white_player).next_move(&board, time);
                        tx.send(bit_move).unwrap();
                        handle.join().unwrap();
                        cli_board.apply_move(bit_move);
                    }
                    Player::Black => {
                        let th_board = board.clone();
                        let th_white_ref = Arc::clone(&white_ref);
                        let th_ponder_ref = Arc::clone(&ponder_ref);
                        let handle = thread::spawn(move || {
                            // Check if move has been played, else ponder
                            while rx.try_recv().is_err() {
                                let mut white_player = th_white_ref.lock().unwrap();
                                let mut ponder_cnt = th_ponder_ref.lock().unwrap();

                                // Ponder
                                white_player.ponder(&th_board);
                                *ponder_cnt += 1;

                                thread::sleep(Duration::from_millis(5));
                            }
                        });
                        let mut black_player = black_ref.lock().unwrap();
                        let bit_move = (*black_player).next_move(&board, time);
                        tx.send(bit_move).unwrap();
                        handle.join().unwrap();
                        cli_board.apply_move(bit_move);
                    }
                }
            }
            BoardState::Win(_) => break,
            BoardState::Draw(_) => break,
        }

        let ponder_cnt = ponder_ref.lock().unwrap();
        let new_time = before.elapsed().unwrap();
        println!(
            "Time needed: {:02}m:{:02}s | Opponent ponders: {}",
            new_time.as_secs() / 60,
            new_time.as_secs() % 60,
            *ponder_cnt
        );
        time = if new_time < min_time {
            min_time
        } else if new_time > max_time {
            max_time
        } else {
            new_time
        }
    }
}
