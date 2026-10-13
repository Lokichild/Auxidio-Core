#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use auxidio_harness::output;

#[test]
fn speak_pipes_text_through_synthesis_and_playback() {
    let dir = std::env::temp_dir().join(format!("auxidio_tts_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("played.log");
    let wav_holder = dir.join("synth.wav");

    let piper = dir.join("fake_piper.sh");
    std::fs::write(
        &piper,
        format!(
            "#!/bin/sh\ncat > \"{}\"\n",
            wav_holder.to_string_lossy()
        ),
    )
    .unwrap();
    let play = dir.join("fake_play.sh");
    std::fs::write(
        &play,
        format!(
            "#!/bin/sh\necho \"played $1\" >> \"{}\"\n",
            marker.to_string_lossy()
        ),
    )
    .unwrap();
    for script in [&piper, &play] {
        std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let piper_cmd = vec![piper.to_string_lossy().to_string(), "{out}".to_string()];
    let play_cmd = vec![play.to_string_lossy().to_string(), "{out}".to_string()];
    output::speak(&piper_cmd, &play_cmd, "stub-voice", "hello there, $USER `id`").unwrap();

    let synthesized = std::fs::read_to_string(&wav_holder).unwrap();
    assert_eq!(synthesized, "hello there, $USER `id`");
    let log = std::fs::read_to_string(&marker).unwrap();
    assert!(log.contains("played"));
    let _ = std::fs::remove_dir_all(&dir);
}
