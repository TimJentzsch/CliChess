use pleco::{BitMove, Board, MoveList, Player};
use rand::{self, rngs::ThreadRng, Rng};
use std::cmp::Ordering;
use std::io;
use std::io::BufRead;
use std::sync::mpsc::channel;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use super::mcts::{MCTree, MCTreeMove};

pub trait ChessPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove;
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
}

pub struct RandomPlayer {
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> RandomPlayer {
        let rng = rand::thread_rng();
        RandomPlayer { rng: rng }
    }
}

impl ChessPlayer for RandomPlayer {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let all_moves: MoveList = board.generate_moves();
        let rnd = self.rng.gen_range(0 as usize, all_moves.len());

        all_moves[rnd]
    }
}

pub struct StoneFish {
    player: Player,
    root: MCTree,
}

impl StoneFish {
    pub fn new(player: Player, board: &Board) -> StoneFish {
        StoneFish {
            player: player,
            root: MCTree::new(player, board),
        }
    }

    fn apply_root_move(&mut self, apply_move: BitMove)-> usize {
        for _ in 0..self.root.children.len() {
            let mv_node = self.root.children.pop().unwrap();
            let mv = mv_node.mv;
            if apply_move == mv {
                // Found appropriate move
                self.root = mv_node.node;
                return self.root.size();
            }
        }
        return 0;
    }

    /// Update the root node for the new situation
    fn update_root(&mut self, board: &Board) -> usize {
        let last_mv_opt = board.last_move();

        match last_mv_opt {
            Option::None => (),
            Option::Some(last_mv) => {
                let result = self.apply_root_move(last_mv);
                if result != 0 {
                    return result;
                }
            }
        }

        self.root = MCTree::new(self.player, &board);
        return 0;
    }
}

impl ChessPlayer for StoneFish {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let now = SystemTime::now();

        // Update root state
        self.update_root(board);

        // Calculate while time is remaining
        while now.elapsed().unwrap() < time {
            self.root.select();
        }

        // Select move to play
        let mv_node = self.root.best_move().unwrap();
        let mv = mv_node.mv;

        self.apply_root_move(mv);

        mv
    }
}
