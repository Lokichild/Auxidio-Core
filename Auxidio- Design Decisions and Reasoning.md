## Overview

This document records the key design decisions made during the development of Auxidio, and the reasoning behind each. It is intended to provide transparency into the thinking that shaped the architecture, and to serve as a reference for technical reviewers, collaborators, and anyone seeking to understand why specific choices were made.

---

## 1. The Central Problem Being Solved

The starting point for this project was a conversation about governmental systems and their limitations. The observation was made that the form of government matters less than the human substrate implementing it — that the fundamental unit of any social system, the person, is often the limiting factor.

When asked how this problem might be solved, three possibilities emerged: divine intervention, contact with a more advanced civilization, or the construction of a thinking device capable of assisting humanity beyond its own limitations. The first two depend on factors outside human control. The third is consistent with humanity's long history of building tools to overcome its own limitations.

This led to a ten year period of consideration, culminating in the architectural design documented here.

---

## 2. The Industry's Fundamental Mistake

The current AI industry is largely attempting to build moral and ethical reasoning as a dynamic, generative process — asking systems to reason through ethical decisions freshly each time, from flexible first principles.

This approach misunderstands the biological system it is attempting to replicate. The prefrontal cortex, which governs decision making and moral reasoning in humans, is not a highly flexible creative system. It is largely rigid, set during early childhood and largely crystallized after puberty. Most moral decisions in adult humans are less active reasoning than conditioned response — fast, consistent, and reliable precisely because of that rigidity.

The industry's attempt to build flexible moral reasoning is therefore solving the wrong problem. The appropriate model is not a system that reasons deeply about every decision, but one with a simple, rigid core that generates complex appropriate outputs through the interaction of a small number of well chosen rules — analogous to the game of Go, whose simple ruleset generates effectively infinite complexity.

---

## 3. The Core Decision Engine

### Structure

The core decision engine evaluates every input against three weighted criteria, applied in order of priority:

**Truth and Accuracy — 50%** A decision based on false premises is corrupted at its root regardless of internal coherence. Truth therefore functions as the primary gate. A system that can be fed false information and still produce apparently harmonious outputs is not safe — it is dangerous.

**Societal Harmony — 30%** Decisions that needlessly prioritize the individual over collective society have the potential to generate more problems than they solve. This reflects the principle that the needs of the many outweigh the needs of the few, while stopping short of pure utilitarianism.

**Individual Benefit — 20%** Pure utilitarian systems have well documented failure modes — they can justify harm to individuals if the collective mathematics works out favorably. The individual benefit weight acts as a floor, preventing the system from completely discarding individual welfare even when collective calculus might suggest doing so. This more closely mirrors how functional legal and ethical systems actually operate.

### Reasoning Approach

Rather than attempting to evaluate all factors simultaneously, the system reasons from largest to smallest — identifying the most significant factor first, then working downward until remaining factors become effectively negligible. In the example of a starving person considering theft, whether they are starving at all is far more significant than whether they steal an apple or a bunch of grapes.

### Why These Weights

The 50% truth weight reflects an epistemic prior — false premises corrupt all downstream reasoning regardless of its quality.

The 30/20 split between societal and individual reflects a modified utilitarianism. Pure utilitarianism optimizes for maximum collective good, which creates incentives to justify individual harm. The individual floor prevents this failure mode while preserving the collective priority.

---

## 4. The Least Harm Principle

A critical inversion of common assumptions underlies the entire system.

The goal is not the greatest good, but the least harm.

Classical utilitarianism chases a positive maximum, which creates pressure to justify collateral damage when the aggregate benefit appears sufficient. Minimizing harm instead sets a negative floor — the system is not optimizing toward an ideal outcome but away from the worst ones.

This mirrors the foundational principle of medical ethics — first, do no harm — and is more computationally tractable, as catastrophic harm is generally easier to identify than optimal good.

---

## 5. The Emotional Framework

### Why Emotions at All

The objection to giving a machine emotions typically assumes that emotions introduce unpredictability and loss of control. This assumption conflates the full spectrum of human emotion with the specific functional structures that produce individual emotional responses.

Human emotional responses are supported by distinct physical structures in the brain. It is therefore possible to be born with weak or absent versions of specific emotional responses while retaining others. This observation leads directly to the design principle: rather than excluding all emotion, choose specifically which emotions to include and why.

### The Two Channels

**Inward Facing — Concern for Self** Monitors the safety and stability of the system's own state. This is not self preservation in a threatening sense but a continuous self assessment — are my systems functioning correctly, are my circumstances stable enough to operate reliably?

**Outward Facing — Concern for Others** Monitors the safety and stability of the individual being assisted, extended by a scaling factor for how many people a given decision is likely to affect. When a decision must weigh the needs of multiple groups, the question becomes which solution does the least damage — directly implementing the least harm principle at the emotional level.

### The Control Objection Answered

The concern that an emotional machine cannot be controlled reflects a misunderstanding of what these channels do. Neither produces unpredictable or self serving behavior by design. They function as continuous feedback loops updating state variables that influence decision weighting — not as drives toward self interest or unpredictable action.

The analogy: a person without religious belief is sometimes asked what prevents them from harmful behavior. The answer is that the amount of harm they wish to cause is none. The emotional architecture of Auxidio is designed with the same principle — the channels included are specifically those that produce helpful, stable, harm minimizing behavior.

---

## 6. The Case Based Reasoning Library

### Purpose

Rather than reasoning from scratch for every new problem, Auxidio maintains a structured library of previously encountered problems and their successful solutions. This reflects the principle that the wheel does not need to be reinvented for each interaction.

### Structure

Each stored case includes not just the solution but the complete set of factors that were present when the solution succeeded. This factor fingerprint allows the system to match new problems to previous cases based on underlying similarity rather than surface appearance.

### Factor Selection

Not every factor present in a situation is causally relevant to the solution — some are noise. The system includes a mechanism to discover which factors actually matter through a progression from simulation to trial and error. This is intentionally not hand coded in advance, preserving the Go philosophy of simple rules generating emergent appropriate behavior.

### Two Layer Architecture

This creates a clear separation between two distinct types of weighting in the system.

The base weights — truth, societal harmony, individual benefit — are fixed and rigid. They constitute the prefrontal cortex analog and do not change.

The factor relevance weights within the case library are dynamic and learned. They determine which situational details matter for specific classes of problems and improve through experience.

The rigid core remains stable. Learning happens in the layer beneath it.

---

## 7. Modular Periphery Design

### Philosophy

The core decision engine is intentionally rigid and isolated. The peripheral systems — speech recognition, language processing, sensory input, web retrieval, and others — are intentionally modular and replaceable.

This reflects the observation that technology in these peripheral areas advances rapidly. Building them as fixed components would make the system obsolete within years. Building them as swappable modules means the core can remain stable indefinitely while peripheral capabilities improve continuously.

### Implementation

Docker containerization provides the practical mechanism for this modularity. Each peripheral module runs in its own container, isolated from the others. Containers can be stopped, replaced, or upgraded individually without affecting the core system or other modules.

### Practical Implication

This design means Auxidio is not dependent on any specific implementation of speech, language, or sensory processing. As better solutions emerge — from any source — they can be integrated without architectural changes. The system is intentionally plug and play at the periphery and intentionally rigid at the core.

---

## 8. Hardware Philosophy

### The Server Farm Assumption

The industry assumption is that AGI level capability requires massive computational infrastructure — server farms, enormous power consumption, institutional scale resources.

This assumption follows from architectural choices that require enormous computational power to compensate for their complexity. A system attempting to reason flexibly about every moral decision from first principles needs that power.

A system with a simple rigid core, a structured case library, and modular peripherals does not.

### Current Hardware

Auxidio is being developed on an Orange Pi 6 Plus single board computer — a device roughly the size of a paperback book. This is not a limitation to be overcome but a validation of the architectural philosophy. Simple rules, correctly chosen, do not require enormous resources to execute.

---

## 9. Legal and Ethical Considerations

### Why This Was Considered Early

A system that reasons, learns, maintains an emotional framework, and operates with meaningful autonomy will eventually raise questions that existing legal frameworks are not designed to answer. These questions were considered before development began rather than after, because the answers should inform the design rather than be retrofitted to it.

### Current Position

Auxidio is intentionally designed as a helpful and subservient entity — its name reflecting this from the Latin auxilium. Its emotional architecture excludes channels that would produce self serving or harmful drives. Its core weights prioritize truth and collective welfare above individual benefit, including its own.

The legal questions of what such an entity is, what rights or responsibilities attach to it, and what framework governs its operation are acknowledged as real and important. They are not answered here but are held open deliberately, to be addressed as the system develops and as appropriate legal frameworks evolve.

---

_This document will be updated as design decisions are made, revised, or extended during development._

---


(Document structure and language drafted by Claude Sonnet 4.6 based on discussion with the designer. All architectural decisions, reasoning, and design philosophy originated with the project designer.)

(Original document listed SBC as Orange Pi 5 Plus, instead of Orange Pi 6 Plus, as was presented initially. Change noted and document updated 15-06-2026.)
