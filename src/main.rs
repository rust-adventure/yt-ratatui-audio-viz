//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use miette::IntoDiagnostic;
use pngtubers::{audio, run_graphics, run_tui, AppState};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::channel;

#[derive(Parser, Debug)]
#[command(version, about = "CPAL record_wav example", long_about = None)]
struct Args {
    /// The audio device to use
    #[arg(short, long, default_value_t = String::from("ZOOM F3 Driver"))]
    device: String,

    /// Use the JACK host
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    #[arg(short, long)]
    #[allow(dead_code)]
    jack: bool,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    env_logger::init();
    let (tx, mut rx) = channel::<Vec<f32>>(100);
    let args = Args::parse();
    let state = Arc::new(Mutex::new(AppState {
        decibels: vec![],
    }));

    let audio_state = state.clone();
    audio::run(&args.device, tx)?;
    let _audio_sample_receiver_task =
        tokio::spawn(async move {
            while let Some(samples) = rx.recv().await {
                let mut s = audio_state.lock().unwrap();
                let max_volume = samples
                    .into_iter()
                    .map(|sample| {
                        let value = (20.0 * sample.log10());
                        if value.is_nan() {
                            -100.0
                        } else {
                            value
                        }
                    })
                    .max_by(|x, y| x.total_cmp(y));
                // dbg!(max_volume);
                s.decibels.push(max_volume.unwrap_or(0.0));
                // println!("got = {}", i.len());
            }
        });

    // run_graphics().await;
    run_tui(state).into_diagnostic().unwrap();

    Ok(())
}
