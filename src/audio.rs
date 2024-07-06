//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BuildStreamError, DeviceNameError, DevicesError,
    PlayStreamError,
};
use cpal::{FromSample, Sample};
use miette::{miette, IntoDiagnostic};
use rustfft::num_complex::ComplexFloat;
use rustfft::{num_complex::Complex, FftPlanner};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

struct FreqRange {
    low: usize,
    high: usize,
}
const BASS: FreqRange = FreqRange { low: 20, high: 140 };
const LOW_MID: FreqRange = FreqRange {
    low: 140,
    high: 400,
};
const MID: FreqRange = FreqRange {
    low: 400,
    high: 2600,
};
const HIGH_MID: FreqRange = FreqRange {
    low: 2600,
    high: 5200,
};
const TREBLE: FreqRange = FreqRange {
    low: 5200,
    high: 14000,
};

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum PngTuberAudioError {
    #[error("hound error")]
    #[diagnostic(code(pngtubers::audio::hound))]
    Hound(#[from] hound::Error),
    #[error("cpal play error")]
    #[diagnostic(code(pngtubers::audio::cpal::play))]
    CpalPlayError(#[from] PlayStreamError),
    #[error("cpal build error")]
    #[diagnostic(code(pngtubers::audio::cpal::build))]
    CpalBuildError(#[from] BuildStreamError),
    #[error("cpal devices error")]
    #[diagnostic(code(pngtubers::audio::cpal::devices))]
    CpalDevicesError(#[from] DevicesError),
    #[error("cpal device name error")]
    #[diagnostic(code(
        pngtubers::audio::cpal::device_name
    ))]
    CpalDeviceNameError(#[from] DeviceNameError),
    #[error("Unsupported Sample Format")]
    #[diagnostic(code(
        pngtubers::audio::unsupported_sample_format
    ))]
    UnsupportedSampleFormat {
        sample_format: cpal::SampleFormat,
        message: String,
    },
}

pub fn run(
    desired_device_name: &str,
    tx: Sender<Vec<f32>>,
) -> miette::Result<(), PngTuberAudioError> {
    // Conditionally compile with jack if the feature is specified.
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    // Manually check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if args.jack {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        )),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = if desired_device_name == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?.find(|x| {dbg!(&x.name());
            x.name()
                .map(|y| y == desired_device_name)
                .unwrap_or(false)
        })
    }
    .expect("failed to find input device");

    println!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");
    println!("Default input config: {:?}", config);

    // The WAV file we're recording to.
    const PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/recorded.wav"
    );
    let spec = wav_spec_from_config(&config);
    // let writer = hound::WavWriter::create(PATH, spec)?;
    // let writer = Arc::new(Mutex::new(Some(writer)));

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    // let writer_2 = writer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(1024);

    let mut buffer = vec![
        Complex {
            re: 0.0f32,
            im: 0.0f32
        };
        1024
    ];
    // dbg!(&config);
    let sample_rate = config.sample_rate().0;
    let nyquist = sample_rate / 2;
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    tx.blocking_send(data.to_vec()).unwrap();
                    // dbg!(&data.len());
                    // println!("{:?}", &data);
                    for (i, v) in data.iter().enumerate() {
                        // let print_num =
                        //     (20.0 * v.log10()).floor();
                        // print!(
                        //     "{:?} ",
                        //     if print_num.is_nan() {
                        //         " ".to_string()
                        //     } else {
                        //         print_num.to_string()
                        //     }
                        // );
                        // println!("{}", v);
                        buffer[i] =
                            Complex::new(*v, 0.0f32);
                    }

                    fft.process(&mut buffer);

                    let results = &buffer
                        .iter()
                        // .take(10)
                        .map(|v| {
                            let n = v.norm();
                            // let n = 20.0 * v.norm().log10();
                            // * (1.0 / 512.0.sqrt());
                            // n.floor() as i32
                            n
                        })
                        .collect::<Vec<f32>>();
                    let results =
                        remove_mirroring(&results);
                    // println!("{:?}", results);
                    let energy_ranges = [
                        BASS, LOW_MID, MID, HIGH_MID,
                        TREBLE,
                    ]
                    .into_iter()
                    .map(|FreqRange { low, high }| {
                        // dbg!(low, high);
                        let low_index = (low as f32
                            / nyquist as f32
                            * results.len() as f32)
                            .round()
                            as usize;
                        // var lowIndex = Math.round((frequency1 / nyquist) * this.freqDomain.length);
                        let high_index = (high as f32
                            / nyquist as f32
                            * results.len() as f32)
                            .round()
                            as usize;
                        // var highIndex = Math.round((frequency2 / nyquist) * this.freqDomain.length);

                        let freq_slice = &results
                            [low_index..=high_index];

                        // var total = 0;
                        let num_frequencies =
                            freq_slice.len();
                        // var numFrequencies = 0;
                        // // add up all of the values for the frequencies
                        let total = results
                            [low_index..=high_index]
                            .iter()
                            .sum::<f32>();
                        // for (var i = lowIndex; i <= highIndex; i++) {
                        //   total += this.freqDomain[i];
                        //   numFrequencies += 1;
                        // }
                        // // divide by total number of frequencies
                        // var toReturn = total / numFrequencies;
                        // dbg!(results.len());
                        total / (num_frequencies as f32)
                    })
                    .collect::<Vec<f32>>();
                    // println!("{:?}", energy_ranges);

                    // write_input_data::<f32, f32>(
                    //     data, &writer_2,
                    // )
                },
                err_fn,
                None,
            )?,
        sample_format => {
            return Err(
                PngTuberAudioError::UnsupportedSampleFormat{
                    sample_format,
                    message: "Unsupported sample format '{sample_format}'".to_string()
                },
            )
        }
        _ => panic!("unsupported cpal::SmapleFormat"),
    };

    stream.play()?;

    // Let recording go for roughly three seconds.
    // std::thread::sleep(std::time::Duration::from_secs(3));
    // drop(stream);
    // writer.lock().unwrap().take().unwrap().finalize()?;
    // println!("Recording {} complete!", PATH);
    Ok(())
}

fn sample_format(
    format: cpal::SampleFormat,
) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(
    config: &cpal::SupportedStreamConfig,
) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config
            .sample_format()
            .sample_size()
            * 8) as _,
        sample_format: sample_format(
            config.sample_format(),
        ),
    }
}

type WavWriterHandle =
    Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(
    input: &[T],
    writer: &WavWriterHandle,
) where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

// any data in the top "half" of the data vec is an alias
// (aka a mirrored exact copy) of the bottom half
// if you took bins 0..10 and 10..20 then data at each
// index:
// 0,1,2,3,4,5,6,7,8,9 == 19,18,17,16,15,14,13,12,11,10
pub fn remove_mirroring(data: &[f32]) -> Vec<f32> {
    let len = data.len() / 2 + 1;
    data[..len].to_vec()
}
