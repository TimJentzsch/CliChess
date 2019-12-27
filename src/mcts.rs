use pleco::{BitMove, Board, MoveList, Player};

use rand::{self, rngs::ThreadRng, Rng};

use std::cmp::Ordering;
use std::sync::{mpsc};
use std::thread;

const SIMULATION_STEPS: usize = 5;

/// The result of a simulation step
pub struct SimResult {
    wins: usize,
    playouts: usize,
}

impl SimResult {
    /// Invert the simulation result
    pub fn invert(&self) -> SimResult{
        let losses = self.playouts - self.wins;
        SimResult{wins: losses, playouts: self.playouts}
    }
}

/// The end of a game
pub enum PlayEnd {
    Win,
    Loss,
}

/// The result of a play
pub enum PlayResult {
    End(PlayEnd),
    Moves(MoveList),
}

impl PlayResult {
    /// Determines the result of a board
    pub fn get_result(board: &Board, player: Player) -> PlayResult {
        let moves = board.generate_moves();

        // Rule 50
        if board.rule_50() >= 50 {
            // The game is a draw
            PlayResult::End(PlayResult::get_draw_result())
        } else if moves.len() == 0 {
            if board.checkmate() {
                // One player wins
                match board.turn() {
                    // The player is in checkmate, the player loses
                    player => PlayResult::End(PlayEnd::Loss),
                    _ => PlayResult::End(PlayEnd::Win),
                }
            } else {
                // The game is a draw
                PlayResult::End(PlayResult::get_draw_result())
            }
        } else {
            // There are moves left to play
            PlayResult::Moves(moves)
        }
    }

    /// Determines the result of a draw
    pub fn get_draw_result() -> PlayEnd {
        // Choose random outcome
        let mut rng = rand::thread_rng();
        let rnd = rng.gen_range(0, 2);
        if rnd == 0 {
            // Win with 50% chance
            PlayEnd::Win
        } else {
            PlayEnd::Loss
        }
    }
}

/// A move to the next node
pub struct MCTreeMove {
    /// The move to reach the node
    pub mv: BitMove,
    /// The next node
    pub node: MCTree,
}

impl Clone for MCTreeMove {
    fn clone(&self) -> Self {
        MCTreeMove {
            mv: self.mv,
            node: self.node.clone(),
        }
    }
}

impl MCTreeMove {
    /// Creates a new MCTree
    pub fn new(mv: BitMove, player: Player, state: &Board) -> MCTreeMove {
        // The new player is the opposite player
        let new_player = match player {
            Player::White => Player::Black,
            Player::Black => Player::White,
        };

        MCTreeMove {
            mv: mv,
            node: MCTree::new(new_player, state),
        }
    }
    /// Compares the play value of the two moves
    pub fn cmp_play_value(&self, other: &MCTreeMove, parent_playouts: usize) -> Ordering {
        let self_value = self.node.play_value();
        let other_value = other.node.play_value();

        if self_value < other_value {
            Ordering::Less
        } else if self_value > other_value {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    /// Compares the selection value of the two plays
    pub fn cmp_select_value(&self, other: &MCTreeMove, parent_playouts: usize) -> Ordering {
        let self_value = self.node.select_value(parent_playouts);
        let other_value = other.node.select_value(parent_playouts);

        if self_value < other_value {
            Ordering::Less
        } else if self_value > other_value {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// Monte-Carlo Tree
pub struct MCTree {
    /// The player to consider
    pub player: Player,
    /// The current state
    pub state: Board,
    /// The number of wins for this state
    pub wins: usize,
    /// The number of playouts for this state
    pub playouts: usize,
    /// The children for this state
    pub children: Vec<MCTreeMove>,
}

impl Clone for MCTree {
    fn clone(&self) -> Self {
        MCTree {
            player: self.player,
            state: self.state.clone(),
            wins: self.wins,
            playouts: self.playouts,
            children: self.children.clone(),
        }
    }
}

impl MCTree {
    /// Creates a new MCTree
    pub fn new(player: Player, state: &Board) -> MCTree {
        // Get the next board
        let state = state.clone();

        MCTree {
            player: player,
            state: state,
            wins: 0,              // No wins yet
            playouts: 0,          // No playouts yet
            children: Vec::new(), // No children yet
        }
    }

    /// Determines the size of the tree
    pub fn size(&self) -> usize {
        let mut size = 1;
        for mv_node in &self.children {
            // Recursively add the size of the child nodes
            size += mv_node.node.size();
        }
        size
    }

    /// Determine the height of the tree
    pub fn height(&self) -> usize {
        if self.is_leaf() {
            // A leaf node has height 0
            0
        } else {
            // Determine the maximum height of its child nodes
            let mut max_height = 0;
            for child in &self.children {
                if child.node.height() > max_height {
                    max_height = child.node.height();
                }
            }
            1 + max_height
        }
    }

    /// Gets the best move, if available
    pub fn best_move(&mut self) -> Option<&MCTreeMove> {
        /* for n in &self.children {
            print!("{}, ", n.node.select_value(self.playouts));
        }
        println!(); */

        if self.is_leaf() {
            // No moves available
            Option::None
        } else {
            // Select the most promising move
            let playouts = self.playouts;
            self.children.sort_by(|a, b| a.cmp_play_value(b, playouts));
            let best_move = self.children.last().unwrap();
            println!(
                "s: {} h: {} p: {} | {}",
                self.size(),
                self.height(),
                self.playouts,
                best_move.node.to_string()
            );
            // self.state.pretty_print();
            // best_move.node.state.pretty_print();
            Option::Some(self.children.last().unwrap())
        }
    }

    /// Updates the current node with the given result
    pub fn update(&mut self, result: &SimResult) {
        self.playouts += result.playouts;
        self.wins += result.wins;
    }

    pub fn to_string(&self) -> String {
        format!("{}/{} ({:05.1}%)", self.wins, self.playouts, self.play_value() * 100.)
    }

    /// Selects the next node to expand
    pub fn select(&mut self) -> SimResult {
        if self.is_leaf() {
            // Leaf nodes can be expanded
            let result = self.expand();
            self.update(&result);
            // Backtrack result
            result
        } else {
            // Select the most promising child node
            let playouts = self.playouts;
            self.children
                .sort_by(|a, b| a.cmp_select_value(b, playouts));
            let node = &mut self.children.last_mut().unwrap().node;
            // The child node has the opposite player, invert the result
            let result = node.select().invert();
            // Update the node
            self.update(&result);
            // Backtrack result
            result
        }
    }

    /// Expands and update the selected node
    pub fn expand(&mut self) -> SimResult {
        let play_result = PlayResult::get_result(&self.state, self.player);

        // Generate child nodes if necessary
        match play_result {
            PlayResult::Moves(moves) => {
                // Generate child nodes
                for mv in moves {
                    let mut new_state = self.state.clone();
                    new_state.apply_move(mv);
                    let node = MCTreeMove::new(mv, self.player, &new_state);
                    self.children.push(node);
                }
            }
            // Backtrack result
            PlayResult::End(end) => (),
        }
        // Make a simulation step and backtrack the result
        self.simulate()
    }

    /// Makes a simulation step for this move
    pub fn simulate(&self) -> SimResult {
        let playouts = SIMULATION_STEPS;
        let (tx, rx) = mpsc::channel();
        // Perform playouts in parallel
        for _ in 0..playouts {
            let board = self.state.clone();
            let player = self.player;
            let tx = tx.clone();
            thread::spawn(move || {
                let result = MCTree::single_playout(board, player);
                tx.send(result).unwrap();
            });
        }

        let mut wins = 0;

        // Aggregate results
        for _ in 0..playouts {
            let result = rx.recv().unwrap();
            match result {
                PlayEnd::Win => wins += 1,
                PlayEnd::Loss => (),
            }
        }
        SimResult{playouts: playouts, wins: wins}
    }

    /// Performs a singular playout
    fn single_playout(board: Board, player: Player) -> PlayEnd {
        let mut board = board.clone();
        // Simulate
        loop {
            // Check for game end
            let result = PlayResult::get_result(&board, player);

            match result {
                PlayResult::Moves(moves) => {
                    // Choose random move
                    let mut rng = rand::thread_rng();
                    let rnd = rng.gen_range(0 as usize, moves.len());
                    let mv = moves[rnd];
                    // Playout with that move
                    board.apply_move(mv);
                }
                PlayResult::End(end) => {
                    // The game ended, return the results
                    return end;
                },
            }
        }
    }

    /// Determines if the node is a leaf node.
    pub fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }

    /// Determines how valuable it is to play this move.
    pub fn play_value(&self) -> f32 {
        if self.playouts == 0 {
            0.5
        } else {
            // Determine 'winrate'
            (self.wins as f32) / (self.playouts as f32)
        }
    }

    /// Determines how valuable it is to expand this node.
    pub fn select_value(&self, parent_playouts: usize) -> f32 {
        // Exploitation: Exploit potentially good moves.
        let exploitation = self.play_value();
        // Exploration: Explore rarely investigated moves.
        let exploration = if self.playouts == 0 {
            1.
        } else {
            let exploration_factor = 1.4142; // sqrt(2)
            exploration_factor * ((parent_playouts as f32).ln() / (self.playouts as f32)).sqrt()
        };
        exploitation + exploration
    }
}
