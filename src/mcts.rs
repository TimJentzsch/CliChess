use pleco::{BitMove, Board, MoveList, Player};

use rand::{self, rngs::ThreadRng, Rng};

use std::cmp::{Ordering, PartialEq};
use std::ops::{Add, AddAssign};
use std::sync::mpsc;
use std::thread;

const PARALLEL_SIMULATIONS: usize = 5;
const PARALLEL_PLAYOUTS: usize = 5;

#[derive(Debug)]
/// The result of a simulation step
pub struct SimResult {
    wins: usize,
    playouts: usize,
}

impl SimResult {
    /// Invert the simulation result
    pub fn invert(&self) -> SimResult {
        let losses = self.playouts - self.wins;
        SimResult {
            wins: losses,
            playouts: self.playouts,
        }
    }
}

impl PartialEq for SimResult {
    fn eq(&self, other: &Self) -> bool {
        self.playouts == other.playouts && self.wins == other.wins
    }
}

impl Add for SimResult {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            wins: self.wins + other.wins,
            playouts: self.playouts + other.playouts,
        }
    }
}

impl AddAssign for SimResult {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            wins: self.wins + other.wins,
            playouts: self.playouts + other.playouts,
        };
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
                if player == board.turn() {
                    // The player is in checkmate, the player loses
                    PlayResult::End(PlayEnd::Loss)
                } else {
                    // The opponent is in checkmate, the player wins
                    PlayResult::End(PlayEnd::Win)
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
    /// Creates a new MCTreeMove
    pub fn new(mv: BitMove, state: &Board) -> MCTreeMove {
        MCTreeMove {
            mv: mv,
            node: MCTree::new(state),
        }
    }
    /// Compares the play value of the two moves
    pub fn cmp_play_value(&self, other: &MCTreeMove) -> Ordering {
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

    /// Determines the move with the maximum play value
    pub fn max_play(moves: &Vec<MCTreeMove>) -> Option<&MCTreeMove> {
        moves.iter().max_by(|a, b| a.cmp_play_value(b))
    }

    /// Determines the move with the maximum play value
    pub fn max_play_mut(
        moves: &mut Vec<MCTreeMove>,
        parent_playouts: usize,
    ) -> Option<&mut MCTreeMove> {
        moves.iter_mut().max_by(|a, b| a.cmp_play_value(b))
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

    /// Determines the move with the maximum select value
    pub fn max_select(moves: &Vec<MCTreeMove>, parent_playouts: usize) -> Option<&MCTreeMove> {
        moves
            .iter()
            .max_by(|a, b| a.cmp_select_value(b, parent_playouts))
    }

    /// Determines the move with the maximum select value
    pub fn max_select_mut(
        moves: &mut Vec<MCTreeMove>,
        parent_playouts: usize,
    ) -> Option<&mut MCTreeMove> {
        moves
            .iter_mut()
            .max_by(|a, b| a.cmp_select_value(b, parent_playouts))
    }
}

/// Monte-Carlo Tree
pub struct MCTree {
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
            state: self.state.clone(),
            wins: self.wins,
            playouts: self.playouts,
            children: self.children.clone(),
        }
    }
}

impl MCTree {
    /// Creates a new MCTree
    pub fn new(state: &Board) -> MCTree {
        // Get the next board
        let state = state.clone();

        MCTree {
            state: state,
            wins: 0,              // No wins yet
            playouts: 0,          // No playouts yet
            children: Vec::new(), // No children yet
        }
    }

    /// The player to consider for this node
    pub fn player(&self) -> Player {
        self.state.turn()
    }

    pub fn assert_valid(&self) {
        if !self.is_leaf() {
            // Validate playout results
            let mut sum_result = SimResult {
                wins: 0,
                playouts: 0,
            };
            for child in &self.children {
                let node = &child.node;
                sum_result += SimResult {
                    wins: node.wins,
                    playouts: node.playouts,
                }
                .invert();

                // Player must be the opposite
                assert_ne!(
                    node.player(), self.player(),
                    "The player must switch every move!"
                );
                // Validate children
                node.assert_valid();
            }
            assert_eq!(
                true,
                self.wins >= sum_result.wins && self.playouts >= sum_result.playouts,
                "This node must have eq or more playouts than its children!"
            );
        }
    }

    pub fn info_str(&self) -> String {
        // Self info
        let size = self.size();
        let height = self.height();
        let width = self.children.len();
        let wins = self.wins;
        let playouts = self.playouts;
        let winrate = (1. - self.play_value()) * 100.; // Inverted for this players
        let s = format!(
            "s:{}, h:{}, w:{}, {}/{} ({:05.1}%)",
            size, height, width, wins, playouts, winrate
        );

        let best_mv = self.best_move();
        match best_mv {
            Option::Some(mv) => {
                // Best move info
                let node = &mv.node;
                let mv_playouts = node.playouts;
                let mv_wins = mv_playouts - node.wins; // Inverted for this player
                // Calculate avg winrate of the available moves
                let mut sum_winrate = 0.;
                for child in &self.children {
                    sum_winrate += child.node.play_value();
                }
                let avg_winrate = sum_winrate / width as f32 * 100.;
                let mv_winrate = node.play_value() * 100.;
                let win_dif = mv_winrate - winrate;
                let avg_win_dif = mv_winrate - avg_winrate;
                format!(
                    "{} | {}/{} ({:05.1}%) => {:+.1}% | avg {:+.1}%",
                    s, mv_wins, mv_playouts, mv_winrate, win_dif, avg_win_dif
                )
            }
            Option::None => s,
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
    pub fn best_move(&self) -> Option<&MCTreeMove> {
        // Select the most promising move
        MCTreeMove::max_play(&self.children)
    }

    /// Updates the current node with the given result
    pub fn update(&mut self, result: &SimResult) {
        self.playouts += result.playouts;
        self.wins += result.wins;
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}/{} ({:05.1}%)",
            self.wins,
            self.playouts,
            (1. - self.play_value()) * 100.
        )
    }

    /// Selects the next node to expand
    pub fn select(&mut self) -> SimResult {
        if self.is_leaf() {
            // Leaf nodes can be expanded
            let result = self.expand();
            // Backtrack result
            result
        } else {
            // Select the most promising child node
            let playouts = self.playouts;
            let best_selection = MCTreeMove::max_select_mut(&mut self.children, playouts).unwrap();
            // The child node has the opposite player, invert the result
            let result = best_selection.node.select().invert();
            // Update the node
            self.update(&result);
            // Backtrack result
            result
        }
    }

    /// Expands and update the selected node
    pub fn expand(&mut self) -> SimResult {
        let play_result = PlayResult::get_result(&self.state, self.player());

        // Generate child nodes if necessary
        match play_result {
            // There are still moves to make
            PlayResult::Moves(moves) => {
                // Generate child nodes
                for mv in moves {
                    let mut new_state = self.state.clone();
                    new_state.apply_move(mv);
                    let node = MCTreeMove::new(mv, &new_state);
                    self.children.push(node);
                }
                // Perform simulations
                let mut result = SimResult {
                    wins: 0,
                    playouts: 0,
                };
                let mut rng = rand::thread_rng();
                for _ in 0..PARALLEL_SIMULATIONS {
                    // Select a child node for simulation
                    let rnd = rng.gen_range(0 as usize, self.children.len());
                    // Make a simulation step
                    let child_result = self.children[rnd].node.simulate().invert();
                    result += child_result;
                }
                self.update(&result);
                result
            }
            // This node is the end of the game, simulate it
            PlayResult::End(_) => self.simulate(),
        }
    }

    /// Makes a simulation step for this move
    pub fn simulate(&mut self) -> SimResult {
        let playouts = PARALLEL_PLAYOUTS;
        let (tx, rx) = mpsc::channel();
        // Perform playouts in parallel
        for _ in 0..playouts {
            let board = self.state.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let result = MCTree::single_playout(board);
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
        let result = SimResult {
            playouts: playouts,
            wins: wins,
        };
        self.update(&result);
        result
    }

    /// Performs a singular playout
    fn single_playout(board: Board) -> PlayEnd {
        let mut board = board.clone();
        let player = board.turn();
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
                }
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
            // Determine 'winrate', but for the opponent
            1. - (self.wins as f32) / (self.playouts as f32)
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
