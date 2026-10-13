use std::process::{Command, Stdio};

pub fn deliver(text: &str) {
    println!("Auxidio: {}", text);
}

pub fn note(text: &str) {
    println!("[harness] {}", text);
}

pub fn default_piper_cmd() -> Vec<String> {
    ["piper", "-m", "{voice}", "--output_file", "{out}"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn default_play_cmd() -> Vec<String> {
    ["aplay", "-q", "-D", "plughw:1,0", "{out}"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// Text reaches piper over stdin: never interpolate model output into a shell
// string (the Phase 7 shell-injection defect stays fixed by construction).
pub fn speak(piper_cmd: &[String], play_cmd: &[String], voice: &str, text: &str) -> std::io::Result<()> {
    let out = std::env::temp_dir().join(format!("auxidio_tts_{}.wav", std::process::id()));
    let out_path = out.to_string_lossy().to_string();

    let synth = crate::sensors::audio::render(
        piper_cmd,
        &[("voice", voice.to_string()), ("out", out_path.clone())],
    );
    let mut child = Command::new(&synth[0])
        .args(&synth[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "piper stdin missing")
        })?;
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        let _ = std::fs::remove_file(&out);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("speech synthesis failed: {}", status),
        ));
    }

    let play = crate::sensors::audio::render(play_cmd, &[("out", out_path.clone())]);
    let played = Command::new(&play[0]).args(&play[1..]).status();
    let _ = std::fs::remove_file(&out);
    match played {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("playback failed: {}", status),
        )),
        Err(err) => Err(err),
    }
}
