use pleco::{BitMove, Board, MoveList, Player};

/// The result of the playouts a node.
pub struct PlayoutResult {
    /// The number of wins for the white player.
    pub white_wins: u32,
    /// The number of wins for the black player.
    pub black_wins: u32,
    /// The number of draws.
    pub draws: u32,
}

impl PlayoutResult {
    pub fn count(&self) -> u32 {
        return self.white_wins + self.black_wins + self.draws;
    }
}

/// A node of the Monte-Carlo-Search-Tree.
pub struct TreeNode {
    /// The current state of the board.
    pub board: Board,
    /// The current playout results for this node.
    pub playout_result: PlayoutResult,
    // The moves available from this position.
    pub moves: Vec<TreeMove>,
}

impl TreeNode {
    /// Determines if the node has not been expanded yet.
    pub fn is_leaf(&self) -> bool {
        self.playout_result.count() == 0 || self.board.checkmate()
    }

    /// Get the player whose turn it is to move.
    pub fn turn(&self) -> Player {
        self.board.turn()
    }

    /// Determines the value of selection of this node.
    pub fn select_value(&self, total_playouts: u32) -> f32 {
        // Node stats
        let wins = match self.turn() {
            Player::White => {self.playout_result.white_wins}
            Player::Black => {self.playout_result.black_wins}
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
                    next_node: TreeNode {
                        board: result_board,
                        playout_result: PlayoutResult {
                            white_wins: 0,
                            black_wins: 0,
                            draws: 0,
                        },
                        moves: vec![],
                    },
                }
            })
            .collect();

        self.moves = tree_moves;
    }

    pub fn select(&mut self) -> PlayoutResult {
        if (self.is_leaf()) {
            // Determine the possible moves
            self.expand();
            // Simulate playouts
            self.simulate()
        } else {
            // Select the most promising node to explore
        }
    }

    pub fn simulate(&mut self) -> PlayoutResult {

    }
}

/// A possible move from a node.
pub struct TreeMove {
    /// The move resulting in the next node.
    pub mv: BitMove,
    /// The node resulting from the move.
    pub next_node: TreeNode,
}
