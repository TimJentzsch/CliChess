use super::mcts::{MCTree, MCTreeMove};
use pleco::{BitMove, Board, MoveList, Player};
use rand::{self, rngs::ThreadRng, Rng};
use std::cmp::Ordering;
use std::io;
use std::io::BufRead;
use std::sync::mpsc::channel;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub trait ChessPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove;
    fn ponder(&mut self, board: &Board);
}

pub struct HumanPlayer {}

impl HumanPlayer {
    pub fn new() -> HumanPlayer {
        HumanPlayer {}
    }
}

impl ChessPlayer for HumanPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let stdin = io::stdin();

        loop {
            let uci_move = stdin.lock().lines().next().unwrap().unwrap();

            let all_moves: MoveList = board.generate_moves();
            let bit_move: Option<BitMove> = all_moves
                .iter()
                .find(|m| m.stringify() == uci_move)
                .cloned();
            if let Some(mov) = bit_move {
                return mov;
            } else {
                println!("Invalid move. Try again:");
            }
        }
    }

    fn ponder(&mut self, board: &Board) {
        thread::sleep(Duration::from_millis(500));
    }
}

pub struct RandomPlayer {}

impl RandomPlayer {
    pub fn new() -> RandomPlayer {
        RandomPlayer {}
    }
}

impl ChessPlayer for RandomPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let all_moves: MoveList = board.generate_moves();
        let mut rng = rand::thread_rng();
        let rnd = rng.gen_range(0 as usize, all_moves.len());
        let mv = all_moves[rnd];

        thread::sleep(time);
        mv
    }

    fn ponder(&mut self, board: &Board) {
        thread::sleep(Duration::from_millis(500));
    }
}

pub struct OldStoneFish {
    player: Player,
    root: MCTree,
}

impl OldStoneFish {
    pub fn new(player: Player, board: &Board) -> OldStoneFish {
        OldStoneFish {
            player: player,
            root: MCTree::new(board),
        }
    }

    /// Tries to apply the given move to the root node
    fn apply_root_move(&mut self, apply_move: BitMove) -> bool {
        for _ in 0..self.root.children.len() {
            let mv_node = self.root.children.pop().unwrap();
            let mv = mv_node.mv;
            if apply_move == mv {
                // Found appropriate move
                self.root = mv_node.node;
                let result = self.root.size();
                println!("{} nodes saved.", result);
                return true;
            }
        }
        return false;
    }

    /// Updates the root node for the new situation
    fn update_root(&mut self, board: &Board) {
        if *board == self.root.state {
            // The root is already up-to-date
            return;
        } else {
            let last_mv_opt = board.last_move();

            match last_mv_opt {
                Option::Some(last_mv) => {
                    // Check if the last move can be applied
                    let result = self.apply_root_move(last_mv);
                    if !result {
                        panic!("Last move can't be applied!");
                    } else {
                        return;
                    }
                }
                Option::None => panic!("No board move found, but board not up-to-date!"),
            }
        }
    }
}

impl ChessPlayer for OldStoneFish {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let now = SystemTime::now();

        assert_eq!(self.player, board.turn(), "Can't move for the opponent!");

        // Update root state
        self.update_root(board);
        assert_eq!(*board, self.root.state, "False move board!");
        assert_eq!(board.turn(), self.root.player(), "Root player not move player!");

        // Calculate while time is remaining
        while now.elapsed().unwrap() < time {
            self.root.select();
        }

        println!("{}", self.root.info_str());

        self.root.assert_valid();

        // Select move to play
        let mv_node = self.root.best_move().unwrap();
        let mv = mv_node.mv;

        self.apply_root_move(mv);

        mv
    }

    fn ponder(&mut self, board: &Board) {
        self.update_root(board);
        assert_eq!(*board, self.root.state, "False ponder board!");
        assert_ne!(self.player, board.turn(), "Must ponder on the opponent's move!");
        assert_eq!(board.turn(), self.root.player(), "Root player not pondering player!");
        self.root.select();
    }
}
