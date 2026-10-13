#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use auxidio_harness::sensors::vision;

fn pgm(pixels: &[u8]) -> Vec<u8> {
    let mut bytes = b"P5\n2 2\n255\n".to_vec();
    bytes.extend_from_slice(pixels);
    bytes
}

#[test]
fn parses_pgm_frames() {
    let (width, height, pixels) = vision::parse_pgm(&pgm(&[1, 2, 3, 4])).unwrap();
    assert_eq!((width, height), (2, 2));
    assert_eq!(pixels, vec![1, 2, 3, 4]);
    assert!(vision::parse_pgm(b"P2\n2 2\n255\n1 2 3 4").is_none());
}

#[test]
fn motion_detector_fires_on_change_only() {
    let mut detector = vision::MotionDetector::new(5.0);
    assert!(!detector.step(&[0, 0, 0, 0]));
    assert!(!detector.step(&[1, 1, 1, 1]));
    assert!(detector.step(&[100, 100, 100, 100]));
    assert!(!detector.step(&[100, 100, 100, 100]));
}

#[test]
fn situation_window_opens_on_motion_and_closes_after_quiet() {
    let mut window = vision::SituationWindow::new(2);
    assert!(!window.note(false, false));
    assert!(window.note(true, false));
    assert!(window.note(false, false));
    assert!(window.is_open());
    assert!(!window.note(false, false));
    assert!(!window.is_open());
    // classifier-forced open (crisis detected in audio/text)
    assert!(window.note(false, true));
    assert!(window.is_open());
}

#[test]
fn capture_command_contract_reads_frames() {
    let dir = std::env::temp_dir().join(format!("auxidio_vision_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("fake_cam.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\nout=\"\"\nfor a in \"$@\"; do out=\"$a\"; done\nprintf 'P5\\n2 2\\n255\\n' > \"$out\"\nprintf '\\001\\002\\003\\004' >> \"$out\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let out = dir.join("frame.pgm");
    vision::capture(
        &[script.to_string_lossy().to_string(), "{out}".to_string()],
        &out.to_string_lossy(),
    )
    .unwrap();
    let (width, height, pixels) = vision::load_gray(&out.to_string_lossy()).unwrap();
    assert_eq!((width, height, pixels), (2, 2, vec![1, 2, 3, 4]));
    let _ = std::fs::remove_dir_all(&dir);
}
