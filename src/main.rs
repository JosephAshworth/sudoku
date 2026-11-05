mod app;

use app::SudokuApp;

fn app_style(_state: &SudokuApp, _theme: &iced::Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: iced::Color::WHITE,
        text_color: iced::Color::BLACK,
    }
}

fn main() -> iced::Result {
    iced::application(SudokuApp::default, SudokuApp::update, SudokuApp::view)
        .title("Sudoku")
        .theme(iced::Theme::Light)
        .style(app_style)
        .centered()
        .run()
}
