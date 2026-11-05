use iced::widget::{button, column, container, row, text, Space};
use iced::widget::Id;
use iced::{Alignment, Element, Length, Point, Rectangle, Task, Theme};
use iced_drop;
use std::collections::HashMap;
use std::sync::OnceLock;

use sudoku::game::{Cell, Game, DEFAULT_STATUS};
use sudoku::grid::{self, Difficulty};

#[derive(Debug, Clone)]
pub enum Screen {
    Menu,
    Playing(Game),
}

pub struct SudokuApp {
    screen: Option<Screen>,
    status_generation: u64,
}

impl Default for SudokuApp {
    fn default() -> Self {
        Self {
            screen: Some(Screen::Menu),
            status_generation: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PickDifficulty(Difficulty),
    NewPuzzle,
    CellClicked { row: usize, col: usize },
    ChooseDigit(u8),
    Undo,
    Hint,
    Drop { digit: u8, point: Point },
    DropZones { digit: u8, zones: Vec<(Id, Rectangle)> },
    RestoreStatus(u64),
}

fn status_color(status: &str) -> iced::Color {
    if status.contains("earned a point") || status.contains("Puzzle complete") {
        iced::Color::from_rgb(0.0, 0.5, 0.15)
    } else if status.contains("lost a point")
        || status.contains("Incorrect")
        || status.contains("Cannot")
        || status.contains("already")
        || status.contains("Select")
        || status.contains("Nothing")
        || status.contains("only")
    {
        iced::Color::from_rgb(0.75, 0.1, 0.1)
    } else {
        iced::Color::BLACK
    }
}

fn hold_point_message(state: &mut SudokuApp) -> Task<Message> {
    let Some(Screen::Playing(game)) = &state.screen else {
        return Task::none();
    };
    let earned = game.status.contains("earned a point");
    let complete = game.complete;
    state.status_generation = state.status_generation.wrapping_add(1);
    let token = state.status_generation;
    if complete || !earned {
        return Task::none();
    }
    Task::future(async move {
        std::thread::sleep(std::time::Duration::from_secs(3));
        Message::RestoreStatus(token)
    })
}

fn cell_ids() -> &'static (HashMap<(usize, usize), Id>, HashMap<Id, (usize, usize)>) {
    static MAPS: OnceLock<(HashMap<(usize, usize), Id>, HashMap<Id, (usize, usize)>)> =
        OnceLock::new();
    MAPS.get_or_init(|| {
        let mut by_coord = HashMap::new();
        let mut by_id = HashMap::new();
        for r in 0..15 {
            for c in 0..15 {
                let id = Id::from(format!("cell-{r}-{c}"));
                by_coord.insert((r, c), id.clone());
                by_id.insert(id, (r, c));
            }
        }
        (by_coord, by_id)
    })
}

fn cell_id(row: usize, col: usize) -> Id {
    cell_ids().0.get(&(row, col)).cloned().expect("cell in range")
}

impl SudokuApp {
    pub fn update(state: &mut Self, message: Message) -> Task<Message> {
        match message {
            Message::PickDifficulty(d) => {
                let puzzle = grid::generate(d);
                state.screen = Some(Screen::Playing(Game::new(puzzle)));
                Task::none()
            }
            Message::NewPuzzle => {
                state.status_generation = state.status_generation.wrapping_add(1);
                state.screen = Some(Screen::Menu);
                Task::none()
            }
            Message::CellClicked { row, col } => {
                if let Some(Screen::Playing(game)) = &mut state.screen {
                    *game = game.clone().select(row, col);
                }
                Task::none()
            }
            Message::ChooseDigit(digit) => {
                if let Some(Screen::Playing(game)) = &mut state.screen {
                    if let Some((row, col)) = game.selected {
                        *game = game.clone().place(row, col, digit);
                    } else {
                        game.status =
                            String::from("Select a cell, then click a number.");
                    }
                }
                hold_point_message(state)
            }
            Message::Undo => {
                if let Some(Screen::Playing(game)) = &mut state.screen {
                    *game = game.clone().undo();
                }
                hold_point_message(state)
            }
            Message::Hint => {
                if let Some(Screen::Playing(game)) = &mut state.screen {
                    *game = game.clone().hint();
                }
                hold_point_message(state)
            }
            Message::Drop { digit, point } => iced_drop::zones_on_point(
                move |zones| Message::DropZones { digit, zones },
                point,
                None,
                None,
            ),
            Message::DropZones { digit, zones } => {
                if let Some(Screen::Playing(game)) = &mut state.screen {
                    if game.complete {
                        return Task::none();
                    }
                    let lookup = &cell_ids().1;
                    for (zone_id, _) in zones {
                        if let Some(&(row, col)) = lookup.get(&zone_id) {
                            *game = game.clone().place(row, col, digit);
                            break;
                        }
                    }
                }
                hold_point_message(state)
            }
            Message::RestoreStatus(token) => {
                if state.status_generation == token {
                    if let Some(Screen::Playing(game)) = &mut state.screen {
                        if !game.complete {
                            game.status = String::from(DEFAULT_STATUS);
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(state: &Self) -> Element<'_, Message> {
        match &state.screen {
            Some(Screen::Menu) => menu_view(),
            Some(Screen::Playing(game)) => playing_view(game),
            None => menu_view(),
        }
    }
}

fn menu_view() -> Element<'static, Message> {
    let choices = column![
        text("Sudoku").size(32),
        Space::new().height(20),
        button("Easy (9×9)").on_press(Message::PickDifficulty(Difficulty::Easy)),
        button("Medium (12×12)").on_press(Message::PickDifficulty(Difficulty::Medium)),
        button("Hard (15×15)").on_press(Message::PickDifficulty(Difficulty::Hard)),
    ]
    .spacing(12)
    .align_x(Alignment::Center);

    container(choices)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn cell_size(difficulty: Difficulty) -> f32 {
    match difficulty {
        Difficulty::Easy => 44.0,
        Difficulty::Medium => 40.0,
        Difficulty::Hard => 36.0,
    }
}

fn box_container<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .style(|_theme: &Theme| container::Style {
            border: iced::Border {
                width: 2.0,
                color: iced::Color::from_rgb(0.25, 0.25, 0.28),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn playing_view(game: &Game) -> Element<'_, Message> {
    let cs = cell_size(game.difficulty);
    let bw = game.difficulty.box_width();
    let bh = game.difficulty.box_height();
    let boxes_down = game.difficulty.boxes_per_col();
    let boxes_across = game.difficulty.boxes_per_row();

    let mut board_rows: Vec<Element<Message>> = Vec::new();
    for box_row in 0..boxes_down {
        let mut box_cols: Vec<Element<Message>> = Vec::new();
        for box_col in 0..boxes_across {
            let mut sub_rows: Vec<Element<Message>> = Vec::new();
            for dr in 0..bh {
                let mut cells: Vec<Element<Message>> = Vec::new();
                let r = box_row * bh + dr;
                for dc in 0..bw {
                    let c = box_col * bw + dc;
                    cells.push(cell_widget(game, r, c, cs));
                }
                sub_rows.push(row(cells).spacing(0).into());
            }
            box_cols.push(box_container(column(sub_rows).spacing(0).into()));
        }
        board_rows.push(row(box_cols).spacing(0).into());
    }

    let board = column(board_rows).spacing(0).align_x(Alignment::Center);

    let digits = game.available_digits();
    let tile = 48.0;
    let tray = row(
        digits
            .into_iter()
            .map(|d| {
                let label = container(text(format!("{d}")).size(24).color(iced::Color::BLACK))
                    .width(Length::Fixed(tile))
                    .height(Length::Fixed(tile))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::WHITE)),
                        border: iced::Border {
                            width: 2.0,
                            color: iced::Color::from_rgb(0.25, 0.25, 0.28),
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    });
                iced_drop::droppable(label)
                    .drag_overlay(true)
                    .on_press(Message::ChooseDigit(d))
                    .on_drop(move |point, _bounds| Message::Drop { digit: d, point })
                    .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(8)
    .align_y(Alignment::Center);

    let instructions = column![
        text("Click a cell, then click a number, or drag a number into a cell.")
            .size(22)
            .width(Length::Fill),
        text("Press Hint to see which numbers cannot go in the selected empty cell.")
            .size(22)
            .width(Length::Fill),
        text("Click a cell you filled, then press Undo to clear it.")
            .size(22)
            .width(Length::Fill),
        text("Press New puzzle to return to the difficulty menu.")
            .size(22)
            .width(Length::Fill),
        text("Red numbers were filled in at the start.")
            .size(22)
            .width(Length::Fill),
        text("Green numbers are ones you placed.")
            .size(22)
            .width(Length::Fill),
    ]
    .spacing(10)
    .width(Length::Fill);

    let left = column![
        text(format!("Score: {}", game.score))
            .size(32)
            .color(iced::Color::BLACK),
        row![
            button("Undo").on_press(Message::Undo),
            button("Hint").on_press(Message::Hint),
            button("New puzzle").on_press(Message::NewPuzzle),
        ]
        .spacing(12),
        text(if game.complete { "Complete!" } else { "" }).size(22),
        text(game.status.clone())
            .size(20)
            .width(Length::Fill)
            .color(status_color(&game.status)),
        instructions,
        text(game.hint_text.clone()).size(20).width(Length::Fill),
    ]
    .spacing(14)
    .width(Length::FillPortion(2))
    .padding(8);

    let right = column![
        container(board)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        container(tray).center_x(Length::Fill).padding([8, 0]),
    ]
    .spacing(12)
    .width(Length::FillPortion(3))
    .height(Length::Fill)
    .align_x(Alignment::Center);

    row![left, right]
        .spacing(24)
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn cell_widget(game: &Game, row: usize, col: usize, cs: f32) -> Element<'_, Message> {
    let selected = game.selected == Some((row, col));
    let (label, number_color) = match game.cells[row][col] {
        Cell::Empty => (String::new(), iced::Color::BLACK),
        Cell::Given(d) => (format!("{d}"), iced::Color::from_rgb(0.75, 0.1, 0.1)),
        Cell::Player(d) => (format!("{d}"), iced::Color::from_rgb(0.0, 0.5, 0.15)),
    };
    let shaded = {
        let box_row = row / game.difficulty.box_height();
        let box_col = col / game.difficulty.box_width();
        (box_row + box_col) % 2 == 1
    };
    let fill = if shaded {
        iced::Color::from_rgb(0.82, 0.82, 0.84)
    } else {
        iced::Color::WHITE
    };

    let content = container(
        text(label)
            .size(if game.difficulty.size() > 9 { 16 } else { 20 })
            .color(number_color),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let cell = button(content)
        .on_press(Message::CellClicked { row, col })
        .width(Length::Fixed(cs))
        .height(Length::Fixed(cs))
        .padding(0)
        .style(move |_theme: &Theme, _status| iced::widget::button::Style {
            background: Some(iced::Background::Color(fill)),
            text_color: number_color,
            border: iced::Border {
                width: if selected { 2.0 } else { 1.0 },
                color: if selected {
                    iced::Color::from_rgb(0.15, 0.15, 0.18)
                } else {
                    iced::Color::from_rgb(0.55, 0.55, 0.58)
                },
                radius: 0.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: true,
        });

    container(cell)
        .width(Length::Fixed(cs))
        .height(Length::Fixed(cs))
        .id(cell_id(row, col))
        .into()
}
