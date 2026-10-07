# Review: "Auxidio: An Architecture for Ethical AGI" (Stephens, Oct 2025)

## 1. What This Document Is

The presentation is the upstream vision statement. The codebase (Phases 1-5) and this repository's design documents are its implementation lineage. This review maps every concept in the deck to an architectural home in the current design set, integrates what is already coherent, and flags what is undefined, underspecified, or in tension with later decisions.

Standing rule from the project's own culture (Roadmap review notes): everything here is proposed, not settled, and submitted for the designer's review.

---

## 2. Concept → Architecture Map

| Deck concept | Architectural home in current design | Status |
|---|---|---|
| Dynamic ethical values (vs static goal maximization) | Two-layer doctrine: rigid 50/30/20 core + learned layer beneath | Implemented in code and docs |
| Sentience — distinguishing self from environment | Self Monitor / inward channel (`Self Monitoring and Maintenance.md`) | Implemented in docs; the deck's C_self gives it a derived score (§4.1) |
| Sapience — others exist outside own needs | Outward channel + Collective Need Score | Outward channel implemented; C_others proposed (§4.2) |
| Consciousness — acting on internalized prompts | Session manager acting on internalized state: maintenance requests, learned adaptations, goals | Implemented in docs |
| Benevolent Steward; Dynamic Executive Function Guide as initial scope | Mission constraint in `System Design.md` §1 (assistive device for mentally disabled users) | Implemented — the deck's low-risk validation scope is exactly the current device |
| C_self (Operational Urgency) | Derived score over self-telemetry | Proposed (§4.1) |
| C_others (Collective Need Score) | Derived score over situation/stress assessment | Proposed, needs device-scope definition (§4.2) |
| P_s priority queue | Harness problem queue when multiple situations compete | Proposed new module (§4.3) |
| MVT / T_Growth safety anchors | Learned safety thresholds | Proposed, scaling question open (§4.4) |
| 110/90/101 accountability loop | Problem-urgency memory, separate from case library | Proposed (§4.5) |
| Locus of Control (consent, earned authority, steward's role) | Safety protocol | Integrated into `System Design.md` (§4.6) |

---

## 3. What the Deck Gets Right Against the Codebase

- **The "arbitrary priorities" critique is real and currently unaddressed.** The Phase 5 system processes one turn at a time with no ranking across competing problems. With continuous situational awareness (multiple people, multiple simultaneous needs), a priority mechanism becomes necessary. P_s is the deck's answer and it fits the harness as a problem queue.
- **"Symbiotic tension" is already validated by the design.** `Self Monitoring and Maintenance.md` §1 states the same principle in plain language: a device that dies mid-crisis harms its user. The deck gives it mathematics (C_self entering the same score as C_others).
- **The deck's initial scope and the repo's mission constraint are the same decision**, made independently in the same direction. Good sign of a coherent lineage.

---

## 4. Integrations

### 4.1 C_self — give the Self Monitor a derived score

The Self Monitor already collects everything the metric lists (hardware health, resource access, data integrity). Add the deck's collapse-to-one-number as a derived metric:

`C_self = 100 − μ(CurrentMetric / PossibleScore)` per metric family, averaged.

Caveat recorded: the deck does not define whether `CurrentMetric` is the raw reading or the deviation from optimal, nor the ×100 scaling on this formula (the C_others formula carries ×100, this one does not). Implementation uses the reading that makes C_self a 0-100 health-urgency scale, documented in code, until the designer supplies the glossary (§5).

### 4.2 C_others — device-scope definition

At societal scale C_others needs population stress data the device does not have. At device scope it is definable and honest: deviation of the *present circle's* assessed stress from the target stress the system considers healthy (T_Growth, learned per the deck). Inputs: outward-channel state tiers, situation classifier output, number of people present (vision). This keeps the metric measurable from local sensors only — consistent with the localization constraint.

### 4.3 P_s — the problem queue

`P_s = (C_self + (C_others × M_impact)) / M_Level`

Home: a `queue` module in the harness. When the session holds more than one open problem (user request, detected situation, own maintenance request), rank by P_s. M_impact = number of people affected (the deck's ethical mandate). M_Level = Maslow level of the problem, 1-5, physiological/safety = 1, so life-and-death divides by 1 and trivialities divide by 5 — matching the deck's stated intent.

Interaction with the rigid core: P_s ranks *what gets attention*; it does not score decisions. Individual decisions still pass through `evaluate_decision`/`score_decision` with the confidence gate. Priority and evaluation are different jobs; both survive.

### 4.4 MVT / T_Growth

- **T_Growth**: learned, not fixed — consistent with the learned-layer doctrine. Home: `ADAPTATIONS` table (a learned parameter like any other, provisional and audited).
- **MVT**: at device scope, "Level 1 crisis across 90% of the population" has no local population. Local analog proposed: the P_s of a simultaneous Level 1 crisis for the user plus everyone currently present. The deck's societal MVT belongs to the future steward scope, not the device. Flagged as a scaling question (§5).

### 4.5 The 110/90/101 accountability loop — separate from the case library

The deck's loop operates on **problem urgency values**; the existing `update_outcome` operates on **solution quality**. They answer different questions and must not share one table:

- Case library: "did this solution work?" — bad solutions are culled (existing behavior, correct).
- Problem records (new `PROBLEM_RECORDS` table): "this problem existed and mattered." Failure re-enters at 110% of last value; success decays to 90%; floor at 101% of the original value so the lesson is never forgotten. This is the deck's permanent memory, and it is compatible with the episodic append-only principle.

Proposed schema addition for `Memory and Weighting.md` once the designer confirms: `PROBLEM_RECORDS(problem_id, description, original_value, current_value, maslow_level, m_impact, recurrence_count, status)`.

### 4.6 Locus of Control — integrated

The deck's three-point safety protocol (consent is core; earned authority; steward proposes and communicates, humans retain executive authority) is already consistent with, and now explicitly cited by, the new "Safety Protocol" section added to `System Design.md`. It also retroactively explains several existing design choices: adaptation announcements with undo, caregiver pins, upgrade requests as proposals, and the system never acting to preserve itself against the user.

### 4.7 Provisional Symbol Glossary (Designer-Confirmable)

The deck's formulas are directionally right and symbolically undefined. The following readings are proposed so planning can proceed; every one enters code as an open testing parameter, so a corrected glossary is a config-level change, not a redesign.

| Symbol | Provisional reading |
|---|---|
| μ | Arithmetic mean across self-metric families (thermal, power, compute, storage, integrity), each normalized to 0..1 |
| CurrentMetric | Absolute deviation of a metric from its optimal band |
| PossibleScore | Deviation ceiling beyond which the metric counts as failed, making CurrentMetric/PossibleScore ∈ 0..1 |
| C_self scaling | The deck writes `100 − μ(...)` without ×100; provisional reading scales to 0-100 so 100 = optimal health |
| P_s^U | Summed urgency of currently observed stressors in the present circle, computed from M_impact/M_Level **alone** — C_self excluded to break the P_s ↔ C_others recursion |
| P_s^TS | The same sum evaluated at the target-stress state T_Growth (nonzero: healthy challenge is the goal, not zero stress) |
| P_s^TC | Normalization ceiling: the local MVT analog (sum under a Level-1 crisis across the present circle) |

---

## 5. Gaps, Tensions, and Questions for the Designer

1. **No truth term in P_s.** — *Incorporated.* `Rust Harness Design.md` §10: P_s ranks attention, the confidence gate decides action kind; `gather_more_data` converts the next action to epistemic (ask/observe/verify) rather than paralyzing; no decisive action on unverified premises at any priority.
2. **Undefined symbols.** — *Provisionally resolved.* Glossary in §4.7, designer-confirmable; config-level so corrections are cheap.
3. **Maslow encoding and conflicts.** — *Incorporated.* Encoding table 1-5 and lowest-level rule in `Rust Harness Design.md` §10; cross-group/level tradeoffs adjudicated by `score_decision`.
4. **MVT scaling.** — *Incorporated.* Local analog (Level-1 crisis across user + present circle) in §4.4; societal MVT deferred to the future steward scope.
5. **110/90/101 vs case culling.** — *Incorporated.* Separate `PROBLEM_RECORDS` table in `Memory and Weighting.md` §8; solution culling and problem permanence no longer share a mechanism.
6. **Dual core vs triple weights.** — *Recorded, no action.* The deck's "dual core" (C_self, C_others) and the code's three weights (truth/collective/individual) are not the same mechanism and do not conflict: one sets priority, the other evaluates decisions. Recorded so nobody later "fixes" the apparent inconsistency by deleting one.

---

## 6. Verdict

The deck is a coherent vision statement whose strongest ideas (dynamic values, symbiotic tension, locus of control, permanent memory) are already present in the implementation lineage in plainer form. Its formulas are the least mature part — directionally right, symbolically undefined — and should enter the codebase only through the glossary-and-differential-test discipline already established for the core. The architecture absorbs every concept in the deck without breaking its rigid-core doctrine: priority is dynamic, evaluation is rigid, truth gates both.
