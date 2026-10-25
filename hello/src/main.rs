mod screen;

use std::process::ExitCode;
use std::time::{Duration, Instant};

use eink::canvas::Canvas;
use eink::epdc::{Epdc, Refresh};
use eink::text::Typeface;
use screen::Facts;

fn uptime() -> Duration {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text: String| {
            text.split_whitespace()
                .next()
                .and_then(|s: &str| s.parse::<f64>().ok())
        })
        .map_or(Duration::ZERO, Duration::from_secs_f64)
}

fn main() -> ExitCode {
    let started: Instant = Instant::now();
    let face: Typeface = match Typeface::regular() {
        Ok(face) => face,
        Err(fault) => {
            eprintln!("hello: font: {fault}");
            return ExitCode::FAILURE;
        }
    };
    let epdc: Epdc = match Epdc::open() {
        Ok(epdc) => epdc,
        Err(fault) => {
            eprintln!("hello: {fault}");
            return ExitCode::FAILURE;
        }
    };
    println!("geometry {:?}", epdc.geometry());
    let canvas: Canvas = screen::render(
        &Facts {
            geometry: epdc.geometry(),
            uptime: uptime(),
        },
        &face,
    );
    let drawn: Duration = started.elapsed();
    match epdc.present(&canvas, std::process::id(), Refresh::Flash) {
        Ok(waited) => {
            println!(
                "drawn in {drawn:?}, refreshed in {:?}, wait returned {waited}",
                started.elapsed() - drawn
            );
            ExitCode::SUCCESS
        }
        Err(fault) => {
            eprintln!("hello: {fault}");
            ExitCode::FAILURE
        }
    }
}
