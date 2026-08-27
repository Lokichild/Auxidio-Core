# Auxidio — Development Roadmap

## Purpose

This document lays out the path from the current Phase 5 state (first integrated test) to a working MVP, and beyond. It is organized as phases with explicit goals, work items, and exit criteria, so progress can be reviewed against something concrete rather than a general sense of forward motion.

The governing constraint throughout is Core Design Principle 6: this is not a race. Each phase is complete when its exit criteria are met and verified, not when it was scheduled to be complete.

---

## Current State (Phase 5)

The integrated conversational loop works end to end: push-to-talk mouse input, Whisper transcription, emotional state analysis, case library lookup, Ollama response generation, and Piper speech output all run together on the target hardware.

However, the reasoning core is not yet genuinely participating in that loop. The decision engine receives hardcoded values, the case library never receives outcome feedback, and several latent defects exist in the integration layer. The work below addresses these in priority order.

---

## Phase 6 — Close the Reasoning Loop (MVP Core)

**Goal:** The decision engine and case library operate on real signals rather than placeholders. This is the defining capability of the system; nothing else qualifies as MVP without it.

### 6.1 Real inputs to the decision engine

`auxidio.py` currently calls `evaluate_decision` with fixed values (0.8 / 0.5 / 0.7). Replace this with an analysis step that extracts actual values from the user's input and context.

- Use the local Ollama model with a strict structured output schema to produce `confidence_rating`, `net_collective_impact`, and `net_individual_impact` from the transcribed input plus any matched case context.
- Validate the extracted values (range checks, fallback behavior on malformed output).
- Keep the decision engine itself untouched — its weights are the rigid core and do not change.

**Exit criteria:** Identical inputs with different emotional or factual content produce different decision scores and tiers.

### 6.2 Outcome feedback loop

`update_outcome` in `case_library.py` exists but is never called; every case is stored with a fixed score of 0.8, so the library cannot learn or cull.

- Add a feedback mechanism. Simplest workable form: a voice confirmation cue after acting on advice ("Did that help?"), mapped to an outcome score.
- Feed that score into `update_outcome` for the case that was matched and used.
- Confirm the removal path works: cases below `MIN_OUTCOME_SCORE` after minimum use count are deleted.

**Exit criteria:** Repeated interactions measurably change case outcome scores over time, and a deliberately poor case is eventually removed.

### 6.3 Wire in `score_decision`

The proportionality engine (`score_decision` in `decision_engine.py`) implements the least harm principle directly — group impact calculation, proportionality threshold, mitigation tiers — and is currently unused.

- Define how a user question maps to a `groups` structure (who is affected, how much, how severely). This is the hardest sub-problem in the phase and should be treated as a design question before code.
- Route decisions affecting multiple parties through `score_decision` instead of `evaluate_decision`.

**Exit criteria:** A question involving a tradeoff between two groups produces a proportionality-aware recommendation (reject / conditional / full proceed), not just a weighted score.

---

## Phase 7 — Defects and Latent Bugs

**Goal:** Remove known defects in the integration layer. These are not design questions; they are fixes.

| Item | Location | Problem | Fix |
|---|---|---|---|
| Shell injection in speech | `auxidio.py` `speak()` | LLM output is interpolated into a `bash -c echo` command; quotes or `$` in a response break or execute it | Pipe text via stdin, never interpolate into a shell string |
| Whisper reloaded per use | `auxidio.py` `transcribe_audio()` | Model loads on every transcription; slow and wasteful on SBC hardware | Load once at startup |
| Audio detection dead code | `auxidio.py` `main()` | Detected device indices are immediately overwritten with hardcoded 0/1 | Use detected values; make hardcoded values a documented fallback only |
| Hardcoded paths and devices | `auxidio.py`, `identity.py`, `case_library.py` | `/mnt/*`, `/home/orangepi`, fixed mouse device path | Move to a config file or environment variables |
| Injection blacklist | `case_library.py` | Substring blacklist is redundant with parameterized queries and will false-positive normal speech ("--", ";") | Remove the pattern scan; keep the security log |

**Exit criteria:** Each item fixed and manually verified on target hardware; no regressions in the conversational loop.

---

## Phase 8 — Usability and Robustness

**Goal:** The system behaves reliably across a real session, not just a single exchange.

- **Conversation memory.** `generate_response` is currently single-turn. Maintain a rolling message history for Ollama so context carries across exchanges.
- **Harden the outward channel.** Keyword matching is brittle to paraphrase. Keep it as the fast path, with the LLM as a fallback classifier for inputs the keywords do not resolve.
- **Error containment.** Timeouts and graceful degradation around Whisper, Ollama, Piper, and audio device failures, so a peripheral failure degrades the session rather than ending it.
- **Startup flow.** The battery confirmation prompt uses blocking `input()`; decide whether this is acceptable for headless operation or needs a non-interactive path.

**Exit criteria:** A multi-turn session survives a simulated peripheral failure and maintains conversational context.

---

## Phase 9 — Structural Work

**Goal:** The architecture claims in the design documents become verifiable facts.

- **Test suite.** `decision_engine.py` and `case_library.py` are pure Python with no hardware dependencies — trivially testable, currently untested. Establish a test runner and cover the decision tiers, proportionality logic, similarity scoring, outcome updates, and security log flow.
- **First real peripheral container.** Move speech I/O out of `auxidio.py` into its own container under `peripherals/`, exercising the Docker modularity claim with a real module instead of the placeholder.
- **Core/peripheral interface.** Define the contract between the core container and peripheral containers before the first separation, so subsequent peripherals follow a pattern rather than each inventing one.

**Exit criteria:** Tests run and pass in the container; at least one peripheral runs in its own container and can be stopped or replaced without touching the core.

---

## Beyond MVP (Deliberately Deferred)

These are not scheduled. They are recorded so their absence is a conscious choice, not an oversight.

- **Factor relevance learning.** Per the design documents, discovering which factors actually matter is intentionally not hand-coded in advance. It becomes relevant once the case library has enough real outcome data to learn from.
- **Legal and ethical positioning.** Acknowledged in the design documents as real and deliberately held open.
- **Hardware portability beyond the Orange Pi 6 Plus.** The architecture should permit it; it is not a near-term goal.

---

## Review Notes

This roadmap reflects the state of the codebase as of the Phase 5 integrated test. Work items reference specific modules and functions to keep them checkable against the code. Ordering is by dependency and priority, not by effort — Phase 6 is the largest and most consequential phase, and Phases 7 through 9 exist to make its output trustworthy.

_(Roadmap drafted by the assistant from a full codebase read at the designer's request. All priorities and exit criteria are proposed, not settled, and are submitted for review.)_
