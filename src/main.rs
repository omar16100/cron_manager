mod app;
mod core;
mod gui;
mod logging;
mod model;

use app::App;

fn main() -> iced::Result {
    logging::init();
    tracing::info!("Cron Manager starting");

    iced::application("Cron Manager", App::update, App::view)
        .theme(App::theme)
        .window_size((900.0, 650.0))
        .centered()
        .run_with(App::new)
}
