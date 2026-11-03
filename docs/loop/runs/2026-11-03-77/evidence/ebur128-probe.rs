use ebur128::{EbuR128, Mode};
fn main() {
    // 0.2 s of a loud tone: too short for one 400 ms gating block.
    let sr = 48000u32; let n = (sr as f32 * 0.2) as usize;
    let d: Vec<f32> = (0..n).map(|i| 0.5 * (std::f32::consts::TAU * 1000.0 * i as f32 / sr as f32).sin()).collect();
    let mut e = EbuR128::new(1, sr, Mode::I | Mode::TRUE_PEAK).unwrap();
    e.add_frames_f32(&d).unwrap();
    println!("short clip I = {:?}", e.loudness_global());
    println!("is_finite = {:?}", e.loudness_global().map(|v| v.is_finite()));
}
