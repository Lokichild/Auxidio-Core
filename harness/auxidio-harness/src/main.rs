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
        let input = line.trim();
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

        match manager.process_turn(input, &model, verifier.as_ref().map(|v| v as &dyn auxidio_harness::verifier::VerifierClient)) {
            Ok(response) => output::deliver(&response),
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
