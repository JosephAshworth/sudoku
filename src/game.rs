use crate::grid::{self, Difficulty, ForbiddenReason, Puzzle};

fn repeat_places(reasons: &[ForbiddenReason]) -> String {
    let mut names = Vec::new();
    if reasons.contains(&ForbiddenReason::Row) {
        names.push("this row");
    }
    if reasons.contains(&ForbiddenReason::Column) {
        names.push("this column");
    }
    if reasons.contains(&ForbiddenReason::Subgrid) {
        names.push("this subgrid");
    }
    match names.as_slice() {
        [] => String::from("this row, column, or subgrid"),
        [only] => (*only).to_string(),
        [first, second] => format!("{first} and {second}"),
        many => {
            let last = many.last().unwrap();
            let rest = many[..many.len() - 1].join(", ");
            format!("{rest}, and {last}")
        }
    }
}

pub const DEFAULT_STATUS: &str = "";
pub const UNDO_PROMPT: &str = "Click the cell you want to undo.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Given(u8),
    Player(u8),
}

#[derive(Debug, Clone)]
pub struct Game {
    pub difficulty: Difficulty,
    pub solution: Vec<Vec<u8>>,
    pub cells: Vec<Vec<Cell>>,
    pub score: i32,
    pub selected: Option<(usize, usize)>,
    pub undoing: bool,
    pub undo_stack: Vec<(usize, usize)>,
    pub status: String,
    pub hint_text: String,
    pub complete: bool,
}

impl Game {
    pub fn new(puzzle: Puzzle) -> Self {
        let size = puzzle.difficulty.size();
        let mut cells = vec![vec![Cell::Empty; size]; size];
        for r in 0..size {
            for c in 0..size {
                if puzzle.givens[r][c] {
                    cells[r][c] = Cell::Given(puzzle.solution[r][c]);
                }
            }
        }
        Self {
            difficulty: puzzle.difficulty,
            solution: puzzle.solution,
            cells,
            score: 0,
            selected: None,
            undoing: false,
            undo_stack: Vec::new(),
            status: String::from(DEFAULT_STATUS),
            hint_text: String::new(),
            complete: false,
        }
    }

    pub fn display_board(&self) -> Vec<Vec<u8>> {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        Cell::Empty => 0,
                        Cell::Given(d) | Cell::Player(d) => *d,
                    })
                    .collect()
            })
            .collect()
    }

    pub fn available_digits(&self) -> Vec<u8> {
        let max = self.difficulty.max_digit();
        let size = self.difficulty.size();
        let mut needed = std::collections::HashMap::new();
        for r in 0..size {
            for c in 0..size {
                if matches!(self.cells[r][c], Cell::Empty) {
                    *needed.entry(self.solution[r][c]).or_insert(0usize) += 1;
                }
            }
        }
        (1..=max)
            .filter(|d| needed.get(d).copied().unwrap_or(0) > 0)
            .collect()
    }

    fn check_complete(&mut self) {
        let size = self.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                match self.cells[r][c] {
                    Cell::Empty => {
                        self.complete = false;
                        return;
                    }
                    Cell::Given(d) | Cell::Player(d) => {
                        if d != self.solution[r][c] {
                            self.complete = false;
                            return;
                        }
                    }
                }
            }
        }
        self.complete = true;
        self.status = String::from("Puzzle complete!");
    }

    pub fn select(self, row: usize, col: usize) -> Self {
        let mut next = self;
        next.selected = if next.selected == Some((row, col)) {
            None
        } else {
            Some((row, col))
        };
        next
    }

    pub fn place(self, row: usize, col: usize, digit: u8) -> Self {
        let mut next = self;
        if next.complete {
            return next;
        }
        match next.cells[row][col] {
            Cell::Given(_) | Cell::Player(_) => {
                next.status = String::from("That cell is already filled.");
                return next;
            }
            Cell::Empty => {}
        }

        let board = next.display_board();
        let correct = next.solution[row][col] == digit;

        if !grid::legal(&board, next.difficulty, row, col, digit) {
            next.score -= 1;
            let reasons = grid::forbidden(&board, next.difficulty, row, col)
                .into_iter()
                .find(|(found, _)| *found == digit)
                .map(|(_, reasons)| reasons)
                .unwrap_or_default();
            let places = repeat_places(&reasons);
            next.status = format!(
                "Cannot place {digit}: it is already in {places}. You lost a point."
            );
            return next;
        }

        if !correct {
            next.score -= 1;
            next.status = format!("Incorrect: {digit} does not belong in that cell. You lost a point.");
            return next;
        }

        next.cells[row][col] = Cell::Player(digit);
        next.undo_stack.push((row, col));
        next.score += 1;
        next.status = String::from("Correct! You earned a point.");
        next.check_complete();
        next
    }

    pub fn begin_undo(self) -> Self {
        if self.undoing {
            return self;
        }
        let mut next = self;
        next.undoing = true;
        next.selected = None;
        next.status = String::from(UNDO_PROMPT);
        next
    }

    pub fn stop_undo(self) -> Self {
        let mut next = self;
        next.undoing = false;
        next.status = if next.complete {
            String::from("Puzzle complete!")
        } else {
            String::from(DEFAULT_STATUS)
        };
        next
    }

    pub fn undo_cell(self, row: usize, col: usize) -> Self {
        let mut next = self;
        if !next.undoing {
            return next;
        }
        if !matches!(next.cells[row][col], Cell::Player(_)) {
            next.status = String::from(
                "You can only undo a cell you filled. Click one of those, or stop undoing.",
            );
            return next;
        }

        next.cells[row][col] = Cell::Empty;
        next.score -= 1;
        next.undo_stack.retain(|&p| p != (row, col));
        next.complete = false;
        next.selected = None;
        next.status = String::from(
            "Undid your entry. You lost a point. Click another cell you filled, or stop undoing.",
        );
        next
    }

    pub fn hint(self) -> Self {
        let mut next = self;
        let (row, col) = match next.selected {
            Some(rc) => rc,
            None => {
                next.status = String::from("Select an empty cell first, then request a hint.");
                return next;
            }
        };

        match next.cells[row][col] {
            Cell::Given(_) | Cell::Player(_) => {
                next.status = String::from("Hints are only for empty cells.");
                return next;
            }
            Cell::Empty => {}
        }

        next.score -= 1;
        let board = next.display_board();
        let forbidden = grid::forbidden(&board, next.difficulty, row, col);

        if forbidden.is_empty() {
            next.hint_text = String::from(
                "No digits appear yet in this cell's row, column, or subgrid. \
                 Digits already placed in those units cannot go here.",
            );
        } else {
            let mut parts = Vec::new();
            for (digit, reasons) in forbidden {
                let mut labels = Vec::new();
                for r in &reasons {
                    labels.push(match r {
                        ForbiddenReason::Row => "row",
                        ForbiddenReason::Column => "column",
                        ForbiddenReason::Subgrid => "subgrid",
                    });
                }
                parts.push(format!(
                    "{digit} (already in this {})",
                    labels.join(", ")
                ));
            }
            next.hint_text = format!("This cell cannot be: {}.", parts.join("; "));
        }
        next.status = String::from("Hint used. You lost a point.");
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid;

    #[test]
    fn undo_refuses_given() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if puzzle.givens[r][c] {
                    let game = Game::new(puzzle.clone())
                        .begin_undo()
                        .undo_cell(r, c);
                    assert!(matches!(game.cells[r][c], Cell::Given(_)));
                    assert_eq!(game.score, 0);
                    assert!(game.undoing);
                    assert!(game.status.contains("only undo a cell you filled"));
                    return;
                }
            }
        }
    }

    #[test]
    fn undo_mode_clears_filled_cell_until_stopped() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if puzzle.givens[r][c] {
                    continue;
                }
                let digit = puzzle.solution[r][c];
                let placed = game.clone().place(r, c, digit);
                if !matches!(placed.cells[r][c], Cell::Player(_)) {
                    continue;
                }
                let prompted = placed.begin_undo();
                assert!(prompted.undoing);
                assert!(prompted.selected.is_none());
                assert_eq!(prompted.status, UNDO_PROMPT);

                let undone = prompted.undo_cell(r, c);
                assert!(matches!(undone.cells[r][c], Cell::Empty));
                assert_eq!(undone.score, 0);
                assert!(undone.undoing);
                assert!(undone.status.contains("lost a point"));

                let stopped = undone.stop_undo();
                assert!(!stopped.undoing);
                assert_eq!(stopped.status, DEFAULT_STATUS);
                assert!(matches!(stopped.cells[r][c], Cell::Empty));
                return;
            }
        }
        panic!("expected a cell the player could fill");
    }

    #[test]
    fn repeat_in_row_rejected() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    continue;
                }
                let digit = puzzle.solution[r][c];
                for c2 in 0..size {
                    if puzzle.givens[r][c2] || c2 == c {
                        continue;
                    }
                    let after = game.clone().place(r, c2, digit);
                    assert!(matches!(after.cells[r][c2], Cell::Empty));
                    assert_eq!(after.score, -1);
                    assert!(
                        after.status.contains("this row"),
                        "status should name the row: {}",
                        after.status
                    );
                    return;
                }
            }
        }
        panic!("expected a row with both a given and an empty cell");
    }

    #[test]
    fn hint_lowers_score() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let mut game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    game = game.select(r, c).hint();
                    assert_eq!(game.score, -1);
                    return;
                }
            }
        }
    }

    #[test]
    fn correct_place_scores() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    let d = puzzle.solution[r][c];
                    let after = game.clone().place(r, c, d);
                    assert!(matches!(after.cells[r][c], Cell::Player(x) if x == d));
                    assert_eq!(after.score, 1);
                    return;
                }
            }
        }
    }

    #[test]
    fn wrong_place_no_fill() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    let wrong = if puzzle.solution[r][c] == 1 {
                        2
                    } else {
                        1
                    };
                    if grid::legal(&game.display_board(), puzzle.difficulty, r, c, wrong) {
                        let after = game.clone().place(r, c, wrong);
                        assert!(matches!(after.cells[r][c], Cell::Empty));
                        assert_eq!(after.score, -1);
                        return;
                    }
                }
            }
        }
    }

    #[test]
    fn hint_no_direct_answer() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    let after = game.clone().select(r, c).hint();
                    assert!(!after.hint_text.to_lowercase().contains("the answer is"));
                    assert_eq!(after.score, -1);
                    return;
                }
            }
        }
    }

    #[test]
    fn tray_removes_exhausted_digit() {
        let puzzle = grid::generate(grid::Difficulty::Easy);
        let mut game = Game::new(puzzle.clone());
        let size = puzzle.difficulty.size();
        let mut digit_counts: std::collections::HashMap<u8, Vec<(usize, usize)>> =
            std::collections::HashMap::new();
        for r in 0..size {
            for c in 0..size {
                if !puzzle.givens[r][c] {
                    digit_counts
                        .entry(puzzle.solution[r][c])
                        .or_default()
                        .push((r, c));
                }
            }
        }
        if let Some((&d, cells)) = digit_counts.iter().find(|(_, v)| v.len() == 1) {
            let (r, c) = cells[0];
            game = game.place(r, c, d);
            assert!(!game.available_digits().contains(&d));
        }
    }
}
