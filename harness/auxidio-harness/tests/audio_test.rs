#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use auxidio_harness::sensors::audio;

fn write_script(dir: &std::path::Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn record_and_transcribe_through_command_contract() {
    let dir = std::env::temp_dir().join(format!("auxidio_audio_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let recorder = write_script(
        &dir,
        "fake_record.sh",
        "#!/bin/sh\nout=\"\"\nfor arg in \"$@\"; do out=\"$arg\"; done\nprintf 'RIFFfakewavdata' > \"$out\"\n",
    );
    let whisper = write_script(
        &dir,
        "fake_whisper.sh",
        "#!/bin/sh\necho 'fake transcript from the stub'\n",
    );

    let out = dir.join("take.wav");
    let record_cmd = vec![recorder, "{out}".to_string()];
    audio::record_to_wav(&record_cmd, &out.to_string_lossy(), 3).unwrap();
    assert!(out.exists());
    assert_eq!(std::fs::read(&out).unwrap(), b"RIFFfakewavdata");

    let whisper_cmd = vec![whisper, "{file}".to_string(), "{model}".to_string()];
    let transcript = audio::transcribe(&whisper_cmd, &out.to_string_lossy(), "stub-model").unwrap();
    assert_eq!(transcript, "fake transcript from the stub");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_substitutes_placeholders() {
    let template: Vec<String> = vec!["cmd".into(), "-d".into(), "{secs}".into(), "{out}".into()];
    let rendered = audio::render(
        &template,
        &[("secs", "7".to_string()), ("out", "/tmp/x.wav".to_string())],
    );
    assert_eq!(rendered, vec!["cmd", "-d", "7", "/tmp/x.wav"]);
}
