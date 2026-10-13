use std::sync::Arc;

use harness_core::ChatMessage;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::config::HarnessConfig;
use crate::context::ContextManager;
use crate::memory;
use crate::model::{ModelClient, ModelError};
use crate::tools;
use crate::verifier::VerifierClient;

fn context_manager_for(config: &HarnessConfig) -> Option<Arc<ContextManager>> {
    if config.context.enabled {
        Some(ContextManager::new(
            config.context.budget_messages,
            config.context.keep_recent_messages,
        ))
    } else {
        None
    }
}

pub struct SessionManager {
    conn: Connection,
    session_id: String,
    context: Vec<ChatMessage>,
    config: HarnessConfig,
    last_episode_id: i64,
    turns_since_checkpoint: u32,
    previous_input: Option<String>,
    resumed: bool,
    context_manager: Option<Arc<ContextManager>>,
    summarizer: Option<Arc<dyn ModelClient + Send + Sync>>,
}

pub type SharedSummarizer = Arc<dyn ModelClient + Send + Sync>;

impl SessionManager {
    pub fn start_or_resume(
        conn: Connection,
        config: &HarnessConfig,
        summarizer: Option<SharedSummarizer>,
    ) -> rusqlite::Result<Self> {
        if let Some(session_id) = memory::open_session_id(&conn)? {
            let checkpoint = memory::load_checkpoint(&conn, &session_id)?;
            let (mut context, mut last_episode_id) = match &checkpoint {
                Some(cp) => (context_from_value(&cp.context), cp.last_episode_id),
                None => (vec![], 0),
            };
            let mut previous_input = None;
            for episode in memory::episodes_after(&conn, &session_id, last_episode_id)? {
                last_episode_id = episode.id;
                match episode.kind.as_str() {
                    "turn_input" => {
                        let text = episode.payload["text"].as_str().unwrap_or_default();
                        context.push(ChatMessage::user(text));
                        previous_input = Some(text.to_string());
                    }
                    "turn_output" => {
                        let text = episode.payload["text"].as_str().unwrap_or_default();
                        context.push(ChatMessage::assistant(text));
                    }
                    _ => {}
                }
            }
            let mut manager = Self {
                conn,
                session_id,
                context: vec![],
                config: config.clone(),
                last_episode_id,
                turns_since_checkpoint: 0,
                previous_input,
                resumed: true,
                context_manager: context_manager_for(config),
                summarizer,
            };
            manager.context = trim_context(context, config.session.context_turns);
            Ok(manager)
        } else {
            let session_id = memory::start_session(&conn)?;
            Ok(Self {
                conn,
                session_id,
                context: vec![],
                config: config.clone(),
                last_episode_id: 0,
                turns_since_checkpoint: 0,
                previous_input: None,
                resumed: false,
                context_manager: context_manager_for(config),
                summarizer,
            })
        }
    }

    pub fn resumed(&self) -> bool {
        self.resumed
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn context_len(&self) -> usize {
        self.context.len()
    }

    pub fn process_turn(
        &mut self,
        user_input: &str,
        model: &dyn ModelClient,
        verifier: Option<&dyn VerifierClient>,
    ) -> rusqlite::Result<String> {
        // Safe point: a background compaction finished since the last turn is
        // swapped in here, between turns, never mid-response.
        if let Some(manager) = &self.context_manager {
            if let Some(compacted) = manager.take_compacted() {
                self.context = compacted;
            }
        }

        let class = crate::classify::classify(&self.conn, Some(model), user_input);
        let episode_id = memory::append_episode(
            &self.conn,
            &self.session_id,
            "turn_input",
            &json!({ "text": user_input }),
            Some(&class.class_id),
        )?;
        self.last_episode_id = episode_id;

        let outward = auxidio_core::emotion::run_outward_channel(
            user_input,
            self.previous_input.as_deref(),
        );
        let clarification_needed = outward["clarification_needed"].as_bool().unwrap_or(false);

        let response = if clarification_needed {
            outward["clarification_question"]
                .as_str()
                .unwrap_or("Could you tell me a little more?")
                .to_string()
        } else {
            let user_state = outward["user_state"].as_str().unwrap_or("stable");
            let case_context = self.case_context(user_state, user_input)?;
            let inputs = crate::weights::extract_decision_inputs(
                model,
                user_input,
                user_state,
                &case_context,
            )
            .unwrap_or_else(|_| crate::weights::conservative_inputs());
            let assessment = auxidio_core::decision::evaluate_decision(
                inputs.confidence_rating,
                inputs.net_collective_impact,
                inputs.net_individual_impact,
            );
            let top_problem = crate::queue::ranked_open_problems(
                &self.conn,
                self.config.queue.target_stress_sum,
                self.config.queue.local_mvt,
            )
            .first()
            .cloned();
            let system =
                self.system_prompt(&outward, &case_context, &assessment, top_problem.as_ref());
            let mut history = self.context.clone();
            history.push(ChatMessage::user(user_input));
            match tools::generate_with_verification(
                model,
                verifier,
                &system,
                &history,
                self.config.verify.max_rounds,
            ) {
                Ok((text, _rounds)) => text,
                Err(ModelError::Unreachable(detail)) => format!(
                    "I could not reach my reasoning model ({}). I have recorded what you said and will try again next turn.",
                    detail
                ),
                Err(err) => format!(
                    "My reasoning model returned an error ({}). I have recorded what you said.",
                    err
                ),
            }
        };

        let output_id = memory::append_episode(
            &self.conn,
            &self.session_id,
            "turn_output",
            &json!({ "text": response }),
            None,
        )?;
        self.last_episode_id = output_id;

        self.context.push(ChatMessage::user(user_input));
        self.context.push(ChatMessage::assistant(&response));
        self.context = trim_context(
            std::mem::take(&mut self.context),
            self.config.session.context_turns,
        );

        if let (Some(manager), Some(summarizer)) = (&self.context_manager, &self.summarizer) {
            manager.maybe_compact(self.context.clone(), summarizer.clone());
        }

        self.turns_since_checkpoint += 1;
        if self.turns_since_checkpoint >= self.config.session.checkpoint_interval {
            self.checkpoint()?;
        }

        self.previous_input = Some(user_input.to_string());
        Ok(response)
    }

    pub fn checkpoint(&mut self) -> rusqlite::Result<()> {
        let context_value = serde_json::to_value(&self.context).unwrap_or(Value::Null);
        memory::save_checkpoint(
            &self.conn,
            &self.session_id,
            self.last_episode_id,
            &context_value,
        )?;
        self.turns_since_checkpoint = 0;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn finish(mut self) -> rusqlite::Result<()> {
        self.checkpoint()?;
        memory::end_session(&self.conn, &self.session_id)
    }

    pub fn finish_into_conn(mut self) -> rusqlite::Result<Connection> {
        self.checkpoint()?;
        memory::end_session(&self.conn, &self.session_id)?;
        Ok(self.conn)
    }

    fn case_context(&self, user_state: &str, user_input: &str) -> rusqlite::Result<String> {
        let query = json!({
            "user_state": user_state,
            "input_length": if user_input.split_whitespace().count() < 10 { "short" } else { "long" }
        });
        let query_map = query.as_object().cloned().unwrap_or_default();
        let similar =
            auxidio_core::cases::find_similar_cases(&self.conn, &query_map, 0.5, 5)?;
        Ok(match similar.first() {
            Some(case) => format!(
                "A similar situation was handled successfully before: {} (confidence: {})",
                case["solution"].as_str().unwrap_or_default(),
                case["outcome_score"].as_f64().unwrap_or(0.0)
            ),
            None => String::new(),
        })
    }

    fn system_prompt(
        &self,
        outward: &Value,
        case_context: &str,
        assessment: &auxidio_core::decision::DecisionResult,
        top_problem: Option<&(crate::memory::ProblemRecord, f64)>,
    ) -> String {
        let style = &outward["communication_style"];
        let concern = match top_problem {
            Some((problem, score)) => format!(
                "Highest-priority open concern right now: {} (priority score {:.2}).",
                problem.description, score
            ),
            None => String::new(),
        };
        format!(
            "You are {name}, a personal reasoning and decision-support companion. You help your user think through decisions clearly and honestly.\n\nCurrent communication guidelines:\n- Tone: {tone}\n- Response length: {length}\n- Elaboration: {elaboration}\n\nDecision engine assessment:\n- Confidence tier: {tier}\n- Decision score: {score}\n\n{case_context}\n{concern}\n\nRespond directly to the user's input following the communication guidelines above. Do not explain your reasoning process. Just respond helpfully and naturally.\n\nIf your response depends on arithmetic, emit a fenced verify block with a claim and a Python script that prints exactly one numeric line, then wait for verification.",
            name = self.config.user_name,
            tone = style["tone"].as_str().unwrap_or("Conversational and thorough."),
            length = style["response_length"].as_str().unwrap_or("As needed."),
            elaboration = style["elaboration"].as_str().unwrap_or("Offered freely."),
            tier = assessment.tier,
            score = assessment.score,
            case_context = case_context,
            concern = concern,
        )
    }
}

fn trim_context(mut context: Vec<ChatMessage>, max_turns: usize) -> Vec<ChatMessage> {
    let max_messages = max_turns.saturating_mul(2);
    if context.len() > max_messages {
        let drop = context.len() - max_messages;
        context.drain(..drop);
    }
    context
}

fn context_from_value(value: &Value) -> Vec<ChatMessage> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}
