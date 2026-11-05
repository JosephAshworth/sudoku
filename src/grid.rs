use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn size(self) -> usize {
        match self {
            Difficulty::Easy => 9,
            Difficulty::Medium => 12,
            Difficulty::Hard => 15,
        }
    }

    pub fn box_width(self) -> usize {
        match self {
            Difficulty::Easy => 3,
            Difficulty::Medium => 4,
            Difficulty::Hard => 5,
        }
    }

    pub fn box_height(self) -> usize {
        3
    }

    pub fn max_digit(self) -> u8 {
        self.size() as u8
    }

    pub fn boxes_per_row(self) -> usize {
        self.size() / self.box_width()
    }

    pub fn boxes_per_col(self) -> usize {
        self.size() / self.box_height()
    }
}

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub difficulty: Difficulty,
    pub solution: Vec<Vec<u8>>,
    pub givens: Vec<Vec<bool>>,
}

pub fn box_origin(difficulty: Difficulty, row: usize, col: usize) -> (usize, usize) {
    let bw = difficulty.box_width();
    let bh = difficulty.box_height();
    (row - row % bh, col - col % bw)
}

pub fn legal(board: &[Vec<u8>], difficulty: Difficulty, row: usize, col: usize, digit: u8) -> bool {
    if digit == 0 || digit > difficulty.max_digit() {
        return false;
    }
    let size = difficulty.size();
    for c in 0..size {
        if c != col && board[row][c] == digit {
            return false;
        }
    }
    for r in 0..size {
        if r != row && board[r][col] == digit {
            return false;
        }
    }
    let (br, bc) = box_origin(difficulty, row, col);
    let bw = difficulty.box_width();
    let bh = difficulty.box_height();
    for r in br..br + bh {
        for c in bc..bc + bw {
            if (r != row || c != col) && board[r][c] == digit {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForbiddenReason {
    Row,
    Column,
    Subgrid,
}

/// Digits already present in this cell's row, column, or subgrid (excluding the cell itself).
pub fn forbidden(
    board: &[Vec<u8>],
    difficulty: Difficulty,
    row: usize,
    col: usize,
) -> Vec<(u8, Vec<ForbiddenReason>)> {
    let size = difficulty.size();
    let mut map: std::collections::BTreeMap<u8, Vec<ForbiddenReason>> =
        std::collections::BTreeMap::new();

    for c in 0..size {
        if c == col {
            continue;
        }
        let d = board[row][c];
        if d != 0 {
            map.entry(d).or_default().push(ForbiddenReason::Row);
        }
    }
    for r in 0..size {
        if r == row {
            continue;
        }
        let d = board[r][col];
        if d != 0 {
            let entry = map.entry(d).or_default();
            if !entry.contains(&ForbiddenReason::Column) {
                entry.push(ForbiddenReason::Column);
            }
        }
    }
    let (br, bc) = box_origin(difficulty, row, col);
    let bw = difficulty.box_width();
    let bh = difficulty.box_height();
    for r in br..br + bh {
        for c in bc..bc + bw {
            if r == row && c == col {
                continue;
            }
            let d = board[r][c];
            if d != 0 {
                let entry = map.entry(d).or_default();
                if !entry.contains(&ForbiddenReason::Subgrid) {
                    entry.push(ForbiddenReason::Subgrid);
                }
            }
        }
    }

    map.into_iter().collect()
}

fn fill_solution(board: &mut [Vec<u8>], difficulty: Difficulty, row: usize, col: usize) -> bool {
    let size = difficulty.size();
    if row == size {
        return true;
    }
    let (next_row, next_col) = if col + 1 == size {
        (row + 1, 0)
    } else {
        (row, col + 1)
    };

    if board[row][col] != 0 {
        return fill_solution(board, difficulty, next_row, next_col);
    }

    let mut digits: Vec<u8> = (1..=difficulty.max_digit()).collect();
    let mut rng = rand::rng();
    digits.shuffle(&mut rng);

    for digit in digits {
        if legal(board, difficulty, row, col, digit) {
            board[row][col] = digit;
            if fill_solution(board, difficulty, next_row, next_col) {
                return true;
            }
            board[row][col] = 0;
        }
    }
    false
}

pub fn generate(difficulty: Difficulty) -> Puzzle {
    let size = difficulty.size();
    let mut solution = vec![vec![0u8; size]; size];
    fill_solution(&mut solution, difficulty, 0, 0);

    let mut givens = vec![vec![false; size]; size];
    let mut rng = rand::rng();
    let box_w = difficulty.box_width();
    let box_h = difficulty.box_height();
    let box_cells = box_w * box_h;
    let min_each = (box_cells as f64 * 0.40).ceil() as usize;
    let max_each = (box_cells as f64 * 0.45).floor() as usize;
    let per_box = if min_each <= max_each {
        rng.random_range(min_each..=max_each)
    } else {
        ((box_cells as f64) * 0.425).round() as usize
    };

    for box_row in 0..difficulty.boxes_per_col() {
        for box_col in 0..difficulty.boxes_per_row() {
            let mut cells: Vec<(usize, usize)> = (0..box_h)
                .flat_map(|dr| {
                    (0..box_w).map(move |dc| (box_row * box_h + dr, box_col * box_w + dc))
                })
                .collect();
            cells.shuffle(&mut rng);
            for (r, c) in cells.into_iter().take(per_box) {
                givens[r][c] = true;
            }
        }
    }

    Puzzle {
        difficulty,
        solution,
        givens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_given_ratio(puzzle: &Puzzle) {
        let total = puzzle.difficulty.size() * puzzle.difficulty.size();
        let givens = puzzle
            .givens
            .iter()
            .flatten()
            .filter(|given| **given)
            .count();
        let ratio = givens as f64 / total as f64;
        assert!(
            (0.40..=0.45).contains(&ratio),
            "givens {givens}/{total} = {ratio}"
        );
        assert_givens_even_across_boxes(puzzle);
    }

    fn assert_givens_even_across_boxes(puzzle: &Puzzle) {
        let difficulty = puzzle.difficulty;
        let box_w = difficulty.box_width();
        let box_h = difficulty.box_height();
        let mut counts = Vec::new();
        for box_row in 0..difficulty.boxes_per_col() {
            for box_col in 0..difficulty.boxes_per_row() {
                let mut count = 0;
                for dr in 0..box_h {
                    for dc in 0..box_w {
                        let r = box_row * box_h + dr;
                        let c = box_col * box_w + dc;
                        if puzzle.givens[r][c] {
                            count += 1;
                        }
                    }
                }
                counts.push(count);
            }
        }
        let min = counts.iter().copied().min().unwrap_or(0);
        let max = counts.iter().copied().max().unwrap_or(0);
        assert!(
            max - min <= 1,
            "subgrid given counts differ by more than 1: {counts:?}"
        );
    }

    fn board_valid(board: &[Vec<u8>], difficulty: Difficulty) -> bool {
        let size = difficulty.size();
        for r in 0..size {
            for c in 0..size {
                let d = board[r][c];
                if d == 0 {
                    continue;
                }
                if !legal(
                    &{
                        let mut b = board.to_vec();
                        b[r][c] = 0;
                        b
                    },
                    difficulty,
                    r,
                    c,
                    d,
                ) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn generate_easy_valid() {
        let p = generate(Difficulty::Easy);
        assert_eq!(p.solution.len(), 9);
        assert!(board_valid(&p.solution, Difficulty::Easy));
        assert_given_ratio(&p);
    }

    #[test]
    fn generate_medium_valid() {
        let p = generate(Difficulty::Medium);
        assert_eq!(p.solution.len(), 12);
        assert!(board_valid(&p.solution, Difficulty::Medium));
        assert_given_ratio(&p);
    }

    #[test]
    fn generate_hard_valid() {
        let p = generate(Difficulty::Hard);
        assert_eq!(p.solution.len(), 15);
        assert!(board_valid(&p.solution, Difficulty::Hard));
        assert_given_ratio(&p);
    }
}
