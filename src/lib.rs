use std::{
    io,
    sync::{Arc, Mutex},
};
use tui::*;
use winit::event_loop::EventLoop;
pub mod audio;
pub mod graphics;
pub mod tui;

pub struct AppState {
    pub decibels: Vec<f32>,
}

pub async fn run_graphics() -> () {
    let event_loop = EventLoop::new();
    let window =
        winit::window::Window::new(&event_loop).unwrap();

    // pollster::block_on(graphics::run(event_loop, window));
    graphics::run(event_loop, window).await
}

pub fn run_tui(
    state: Arc<Mutex<AppState>>,
) -> Result<(), io::Error> {
    let mut terminal = setup_terminal()?;
    run(&mut terminal, state)?;
    restore_terminal(&mut terminal)?;
    Ok(())
}
