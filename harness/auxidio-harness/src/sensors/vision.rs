use std::process::Command;

pub fn default_capture_cmd() -> Vec<String> {
    ["fswebcam", "-q", "-d", "/dev/video0", "-r", "320x240", "--no-banner", "{out}"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn capture(capture_cmd: &[String], out_path: &str) -> std::io::Result<()> {
    let argv = crate::sensors::audio::render(
        capture_cmd,
        &[("out", out_path.to_string())],
    );
    let status = Command::new(&argv[0]).args(&argv[1..]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("capture command failed: {}", status),
        ))
    }
}

// Frames travel as PGM (P5): a one-header grayscale image any capture command
// can emit and the harness can diff without an image crate.
pub fn parse_pgm(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    fn read_token(cursor: &mut &[u8]) -> Option<Vec<u8>> {
        while let Some(&c) = cursor.first() {
            if c.is_ascii_whitespace() {
                *cursor = &cursor[1..];
            } else {
                break;
            }
        }
        let mut token = vec![];
        while let Some(&c) = cursor.first() {
            if c.is_ascii_whitespace() {
                *cursor = &cursor[1..];
                break;
            }
            token.push(c);
            *cursor = &cursor[1..];
        }
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    }

    let mut cursor = bytes.strip_prefix(b"P5")?;
    let width_token = read_token(&mut cursor)?;
    let height_token = read_token(&mut cursor)?;
    let maxval_token = read_token(&mut cursor)?;
    if maxval_token != b"255" {
        return None;
    }
    let width: usize = String::from_utf8(width_token).ok()?.parse().ok()?;
    let height: usize = String::from_utf8(height_token).ok()?.parse().ok()?;
    if cursor.len() < width * height {
        return None;
    }
    Some((width, height, cursor[..width * height].to_vec()))
}

pub fn load_gray(path: &str) -> std::io::Result<(usize, usize, Vec<u8>)> {
    let bytes = std::fs::read(path)?;
    parse_pgm(&bytes).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "not a P5 pgm frame")
    })
}

pub struct MotionDetector {
    pub threshold: f64,
    last: Option<Vec<u8>>,
}

impl MotionDetector {
    pub fn new(threshold: f64) -> Self {
        Self { threshold, last: None }
    }

    pub fn step(&mut self, frame: &[u8]) -> bool {
        let motion = match &self.last {
            Some(previous) if previous.len() == frame.len() => {
                let diff: u64 = previous
                    .iter()
                    .zip(frame.iter())
                    .map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs() as u64)
                    .sum();
                let mean = diff as f64 / frame.len().max(1) as f64;
                mean > self.threshold
            }
            _ => false,
        };
        self.last = Some(frame.to_vec());
        motion
    }
}

pub struct SituationWindow {
    open: bool,
    quiet_count: u32,
    quiet_to_close: u32,
}

impl SituationWindow {
    pub fn new(quiet_to_close: u32) -> Self {
        Self { open: false, quiet_count: 0, quiet_to_close }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn note(&mut self, motion: bool, forced: bool) -> bool {
        if motion || forced {
            self.open = true;
            self.quiet_count = 0;
        } else if self.open {
            self.quiet_count += 1;
            if self.quiet_count >= self.quiet_to_close {
                self.open = false;
                self.quiet_count = 0;
            }
        }
        self.open
    }
}
