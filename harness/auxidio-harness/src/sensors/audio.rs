use std::process::Command;

// Sensor capture is command-driven on purpose: the capture binaries are the
// swappable peripherals (arecord/whisper-cli today, anything with the same
// contract tomorrow), and the harness stays free of ALSA/V4L2 link deps.

pub fn default_record_cmd() -> Vec<String> {
    ["arecord", "-q", "-d", "{secs}", "-r", "44100", "-c", "1", "-f", "S16_LE", "{out}"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn default_whisper_cmd() -> Vec<String> {
    ["whisper-cli", "-q", "-m", "{model}", "-f", "{file}", "-otxt", "-of", "/dev/stdout"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn render(template: &[String], vars: &[(&str, String)]) -> Vec<String> {
    template
        .iter()
        .map(|token| {
            let mut out = token.clone();
            for (key, value) in vars {
                out = out.replace(&format!("{{{}}}", key), value);
            }
            out
        })
        .collect()
}

pub fn record_to_wav(
    record_cmd: &[String],
    out_path: &str,
    seconds: u32,
) -> std::io::Result<()> {
    let argv = render(record_cmd, &[("secs", seconds.to_string()), ("out", out_path.to_string())]);
    let status = Command::new(&argv[0]).args(&argv[1..]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("record command failed: {}", status),
        ))
    }
}

pub fn transcribe(whisper_cmd: &[String], file: &str, model: &str) -> std::io::Result<String> {
    let argv = render(whisper_cmd, &[("file", file.to_string()), ("model", model.to_string())]);
    let output = Command::new(&argv[0]).args(&argv[1..]).output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("whisper command failed: {}", output.status),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
