use pleco::{BitMove, Board, MoveList, Player};
use rand::{self, Rng};
use std::cmp::Ordering;

use super::ChessPlayer;

use std::time::{Duration, SystemTime};
use std::ops::{Add, AddAssign};
use std::sync::mpsc;
use std::thread;

pub struct StoneFish {
    player: Player,
    root: TreeNode,
}

impl StoneFish {
    pub fn new(player: Player, board: &Board) -> StoneFish {
        StoneFish {
            player: player,
            root: TreeNode::new(board.clone()),
        }
    }

    /// Tries to apply the given move to the root node
    fn apply_root_move(&mut self, apply_move: BitMove) -> bool {
        for _ in 0..self.root.moves.len() {
            let mv_node = self.root.moves.pop().unwrap();
            let mv = mv_node.mv;
            if apply_move == mv {
                // Found appropriate move
                self.root = mv_node.next_node;
                // let result = self.root.size();
                // println!("{} nodes saved.", result);
                return true;
            }
        }
        return false;
    }

    /// Updates the root node for the new situation
    fn update_root(&mut self, board: &Board) {
        if *board == self.root.board {
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

impl ChessPlayer for StoneFish {
    fn next_move(&mut self, board: &Board, time: Duration) -> BitMove {
        let now = SystemTime::now();

        assert_eq!(self.player, board.turn(), "Can't move for the opponent!");

        // Update root state
        self.update_root(board);
        assert_eq!(*board, self.root.board, "False move board!");
        assert_eq!(board.turn(), self.root.turn(), "Root player not move player!");

        // Calculate while time is remaining
        while now.elapsed().unwrap() < time {
            self.root.select();
        }

        // println!("{}", self.root.info_str());

        // self.root.assert_valid();

        // Select move to play
        let mv_node = self.root.best_move();
        let mv = mv_node.mv;

        self.apply_root_move(mv);

        mv
    }

    fn ponder(&mut self, board: &Board) {
        self.update_root(board);
        assert_eq!(*board, self.root.board, "False ponder board!");
        assert_ne!(self.player, board.turn(), "Must ponder on the opponent's move!");
        assert_eq!(board.turn(), self.root.turn(), "Root player not pondering player!");
        self.root.select();
    }
}


/// The result of the playouts a node.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
pub struct PlayoutResult {
    /// The number of wins for the white player.
    pub white_wins: u32,
    /// The number of wins for the black player.
    pub black_wins: u32,
    /// The number of draws.
    pub draws: u32,
}

impl PlayoutResult {
    pub fn new(white_wins: u32, black_wins: u32, draws: u32) -> PlayoutResult {
        PlayoutResult {
            white_wins,
            black_wins,
            draws,
        }
    }

    pub fn new_empty() -> PlayoutResult {
        PlayoutResult::new(0, 0, 0)
    }

    pub fn count(&self) -> u32 {
        return self.white_wins + self.black_wins + self.draws;
    }
}

impl Add for PlayoutResult {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            white_wins: self.white_wins + other.white_wins,
            black_wins: self.black_wins + other.black_wins,
            draws: self.draws + other.draws,
        }
    }
}

impl AddAssign for PlayoutResult {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            white_wins: self.white_wins + other.white_wins,
            black_wins: self.black_wins + other.black_wins,
            draws: self.draws + other.draws,
        };
    }
}

/// A node of the Monte-Carlo-Search-Tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// The current state of the board.
    pub board: Board,
    /// The current playout results for this node.
    pub playout_result: PlayoutResult,
    // The moves available from this position.
    pub moves: Vec<TreeMove>,
}

impl TreeNode {
    pub fn new(board: Board) -> TreeNode {
        TreeNode {
            board,
            playout_result: PlayoutResult::new_empty(),
            moves: vec![],
        }
    }

    /// Determine if the node has not been expanded yet.
    pub fn is_leaf(&self) -> bool {
        self.playout_result.count() == 0 || self.board.checkmate()
    }

    /// Get the player whose turn it is to move.
    pub fn turn(&self) -> Player {
        self.board.turn()
    }

    /// Get the total number of playouts for this node.
    pub fn playouts(&self) -> u32 {
        self.playout_result.count()
    }

    pub fn best_move(&self) -> TreeMove {
        // Select the most promising move to play
        let best_move = self.moves.iter().max_by(|mv1, mv2| {
            if mv1.playout_value() < mv2.playout_value() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }).unwrap();

        best_move.clone()
    }

    /// Get the value to play this node.
    pub fn play_value(&self) -> f32 {
        // Node stats
        let wins = match self.turn() {
            Player::White => self.playout_result.white_wins,
            Player::Black => self.playout_result.black_wins,
        };
        let draws = self.playout_result.draws;
        let playouts = self.playout_result.count();

        // Exploit moves with a good winrate
        let exploitation = ((wins as f32) + (draws as f32) / 2.0) / (playouts as f32);

        exploitation
    }

    /// Get the value of selection of this node.
    pub fn select_value(&self, total_playouts: u32) -> f32 {
        // Node stats
        let wins = match self.turn() {
            Player::White => self.playout_result.white_wins,
            Player::Black => self.playout_result.black_wins,
        };
        let draws = self.playout_result.draws;
        let playouts = self.playout_result.count();

        // Exploit moves with a good winrate
        let exploitation = ((wins as f32) + (draws as f32) / 2.0) / (playouts as f32);

        // Exploration parameter = sqrt(2)
        let c = 1.41421356;

        // Explore moves with few playouts
        let exploration = c * ((total_playouts as f32).ln() / (playouts as f32)).sqrt();

        exploitation + exploration
    }

    /// Expand the node to determine the possible moves.
    pub fn expand(&mut self) {
        assert!(self.is_leaf());

        let moves = self.board.generate_moves();

        let tree_moves: Vec<TreeMove> = moves
            .iter()
            .map(|mv| {
                let mut result_board = self.board.clone();
                result_board.apply_move(*mv);

                TreeMove {
                    mv: *mv,
                    next_node: TreeNode::new(result_board),
                }
            })
            .collect();

        self.moves = tree_moves;
    }

    /// Select the most promising node to explore
    pub fn select(&mut self) -> PlayoutResult {
        let result = if self.is_leaf() {
            // Determine the possible moves
            self.expand();
            // Simulate playouts
            self.simulate()
        } else {
            let total_playouts = self.playouts();
            // Select the most promising node to explore
            let best_move = self.moves.iter_mut().max_by(|mv1, mv2| {
                if mv1.select_value(total_playouts) < mv2.select_value(total_playouts) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }).unwrap();

            // Propagate the selection until a leaf node is reached
            best_move.select()
        };

        // Update playouts
        self.playout_result += result;
        // Backtrack
        result
    }

    /// Simulate the value of the given node.
    pub fn simulate(&mut self) -> PlayoutResult {
        let playouts = 8;

        let (tx, rx) = mpsc::channel();

        // Perform playouts in parallel
        for _ in 0..playouts {
            let board = self.board.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let result = TreeNode::playout(board);
                tx.send(result).unwrap();
            });
        }

        let mut total_result = PlayoutResult::new_empty();

        // Aggregate results
        for _ in 0..playouts {
            let result = rx.recv().unwrap();
            
            total_result += result;
        }
    
        total_result
    }

    /// Determine the value to play the given move
    pub fn playout_value(board: &Board, mv: &BitMove) -> i32 {
        let mut rng = rand::thread_rng();

        // Exploit good captures
        let exploitation = match board.captured_piece(*mv) {
            pleco::PieceType::None => { 0 }
            pleco::PieceType::P => { 1 }
            pleco::PieceType::N => { 3 }
            pleco::PieceType::B => { 3 }
            pleco::PieceType::R => { 5 }
            pleco::PieceType::Q => { 9 }
            pleco::PieceType::K => { 100 }
            pleco::PieceType::All => { 0 }
        };

        // Explore other possibilities
        let exploration = rng.gen_range(0, 40);

        exploitation + exploration
    }

    // Playout a board semi-randomly
    pub fn playout(mut board: Board) -> PlayoutResult {
        // Simulate
        loop {
            // Check for game end
            if board.checkmate() {
                return match board.turn() {
                    // White can't move, black wins
                    Player::White => { PlayoutResult::new(0, 1, 0) }
                    // Black can't move, white wins
                    Player::Black => { PlayoutResult::new(1, 0, 0) }
                }
            } else if board.rule_50() >= 50 || board.stalemate() {
                return PlayoutResult::new(0, 0, 1);
            } else {
                // Generate moves
                let moves = board.generate_moves();
                
                assert!(moves.len() > 0);
                
                // Chose best move
                let mut best_value = TreeNode::playout_value(&board, &moves[0]);
                let mut best_move = moves[0];

                for i in 1..moves.len() {
                    let value = TreeNode::playout_value(&board, &moves[i]);
                    if value > best_value {
                        best_value = value;
                        best_move = moves[i];
                    }
                }

                // Play the best move
                board.apply_move(best_move);
            }
        }
    }
}

/// A possible move from a node.
#[derive(Debug, Clone)]
pub struct TreeMove {
    /// The move resulting in the next node.
    pub mv: BitMove,
    /// The node resulting from the move.
    pub next_node: TreeNode,
}

impl TreeMove {
    pub fn select_value(&self, total_playouts: u32) -> f32 {
        self.next_node.select_value(total_playouts)
    }

    pub fn playout_value(&self) -> f32 {
        self.next_node.play_value()
    }

    pub fn select(&mut self) -> PlayoutResult {
        self.next_node.select()
    }
}
