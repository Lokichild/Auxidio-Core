use std::io::{BufRead, Write};

use auxidio_harness::config;
use auxidio_harness::memory;
use auxidio_harness::model::OllamaClient;
use auxidio_harness::output;
use auxidio_harness::session::SessionManager;
use auxidio_harness::verifier::PythonVerifier;

fn main() {
    let config = config::load("harness.toml");

    let conn = match memory::open_memory(&config.db_path) {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("cannot open memory store at {}: {}", config.db_path, err);
            std::process::exit(1);
        }
    };

    if let Err(err) = auxidio_harness::classify::seed_default_classes(&conn) {
        output::note(&format!("could not seed situation classes: {}", err));
    }

    let summarizer: Option<auxidio_harness::session::SharedSummarizer> = if config.context.enabled {
        Some(std::sync::Arc::new(OllamaClient::new(
            &config.ollama.url,
            &config.context.summarizer_model,
            config.ollama.timeout_secs,
        )))
    } else {
        None
    };

    let mut manager = match SessionManager::start_or_resume(conn, &config, summarizer) {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("cannot start session: {}", err);
            std::process::exit(1);
        }
    };

    let selfmon = if config.selfmon.enabled {
        match auxidio_harness::selfmon::start(
            &config.db_path,
            std::time::Duration::from_secs(config.selfmon.interval_secs),
            auxidio_harness::output::deliver,
        ) {
            Ok(handle) => {
                output::note("self monitor started");
                Some(handle)
            }
            Err(err) => {
                output::note(&format!("self monitor unavailable: {}", err));
                None
            }
        }
    } else {
        None
    };

    println!("=== Auxidio Harness ===");
    if manager.resumed() {
        output::note(&format!(
            "session {} resumed with {} context messages — picking up where we left off",
            manager.session_id(),
            manager.context_len()
        ));
    } else {
        output::note(&format!("new session {}", manager.session_id()));
    }
    println!("I am {}. Type a message, or /exit to end the session.", config.user_name);

    let model = OllamaClient::new(
        &config.ollama.url,
        &config.ollama.model,
        config.ollama.timeout_secs,
    );

    let verifier = if config.verify.enabled {
        match PythonVerifier::spawn(&config.verify.python_cmd, &config.verify.service_path) {
            Ok(verifier) => {
                output::note("math verifier service started");
                Some(verifier)
            }
            Err(err) => {
                output::note(&format!(
                    "math verifier unavailable ({}); claims will be reported unverified",
                    err
                ));
                None
            }
        }
    } else {
        None
    };

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        print!("> ");
        let _ = stdout.flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let input = line.trim().to_string();
        if input.is_empty() {
            continue;
        }
        if input == "/exit" {
            break;
        }
        if input == "/checkpoint" {
            match manager.checkpoint() {
                Ok(()) => output::note("checkpoint written"),
                Err(err) => output::note(&format!("checkpoint failed: {}", err)),
            }
            continue;
        }

        let turn_input = if let Some(secs) = input.strip_prefix("/talk") {
            if !config.audio.enabled {
                output::note("audio input is disabled ([audio] enabled = true to use it)");
                continue;
            }
            let seconds: u32 = secs.trim().parse().unwrap_or(5);
            let wav = std::env::temp_dir()
                .join(format!("auxidio_take_{}.wav", std::process::id()));
            let wav_path = wav.to_string_lossy().to_string();
            let transcript = match auxidio_harness::sensors::audio::record_to_wav(
                &config.audio.record_cmd,
                &wav_path,
                seconds,
            ) {
                Ok(()) => auxidio_harness::sensors::audio::transcribe(
                    &config.audio.whisper_cmd,
                    &wav_path,
                    &config.audio.whisper_model,
                ),
                Err(err) => Err(err),
            };
            let _ = std::fs::remove_file(&wav);
            match transcript {
                Ok(text) if !text.is_empty() => {
                    println!("You said: {}", text);
                    text
                }
                Ok(_) => {
                    output::note("no speech detected");
                    continue;
                }
                Err(err) => {
                    output::note(&format!("audio path failed: {}", err));
                    continue;
                }
            }
        } else {
            input
        };

        match manager.process_turn(
            &turn_input,
            &model,
            verifier.as_ref().map(|v| v as &dyn auxidio_harness::verifier::VerifierClient),
        ) {
            Ok(response) => {
                output::deliver(&response);
                if config.tts.enabled {
                    if let Err(err) = output::speak(
                        &config.tts.piper_cmd,
                        &config.tts.play_cmd,
                        &config.tts.voice,
                        &response,
                    ) {
                        output::note(&format!("speech output failed: {}", err));
                    }
                }
            }
            Err(err) => output::note(&format!("turn failed: {}", err)),
        }
    }

    match manager.finish() {
        Ok(()) => output::note("session closed and checkpointed"),
        Err(err) => output::note(&format!("session close failed: {}", err)),
    }
    if let Some(handle) = selfmon {
        handle.stop();
    }
}
