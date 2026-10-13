import json
import os
import random
import re
import sys
import tempfile
import types

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, REPO_ROOT)

# The vectors exercise the outward channel, decision engine, and case library.
# Inward-channel hardware reads are never invoked here, so a stub keeps the
# generator runnable on machines without psutil. The frozen reference is untouched.
try:
    import psutil  # noqa: F401
except ModuleNotFoundError:
    _stub = types.ModuleType("psutil")
    _stub.sensors_temperatures = lambda: {}
    _stub.virtual_memory = lambda: types.SimpleNamespace(percent=0.0)
    sys.modules["psutil"] = _stub

import case_library
import decision_engine
import emotional_framework

OUT_DIR = os.path.join(REPO_ROOT, "core", "tests", "vectors")
ISO_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?")


def mask(value):
    if isinstance(value, str):
        return ISO_RE.sub("<TS>", value)
    if isinstance(value, list):
        return [mask(v) for v in value]
    if isinstance(value, dict):
        return {k: mask(v) for k, v in value.items()}
    return value


def gen_decision(rng):
    vectors = {"tier": [], "evaluate": [], "tier_coefficient": [], "group_impact": [], "score_decision": []}

    boundaries = [0.0, 0.30, 0.3099999, 0.31, 0.55, 0.5599999, 0.56, 0.85, 0.8599999, 0.86, 1.0]
    for value in boundaries + [rng.uniform(0, 1) for _ in range(60)]:
        vectors["tier"].append(
            {"input": value, "expected": decision_engine.get_confidence_tier(value)}
        )

    for _ in range(120):
        c = rng.uniform(0, 1)
        col = rng.uniform(-1, 1)
        ind = rng.uniform(-1, 1)
        vectors["evaluate"].append(
            {
                "confidence": c,
                "collective": col,
                "individual": ind,
                "expected": decision_engine.evaluate_decision(c, col, ind),
            }
        )

    for tier in [0.0, 0.5, 1.0, 2.0, 3.0]:
        for exponent in [1.0, 1.5, 2.0, 2.5]:
            vectors["tier_coefficient"].append(
                {"tier": tier, "exponent": exponent,
                 "expected": decision_engine.tier_coefficient(tier, exponent)}
            )

    groups = [
        {"name": "a", "size": 1, "harm_tier": 0, "benefit_tier": 2},
        {"name": "b", "size": 3, "harm_tier": 1, "benefit_tier": 0},
        {"name": "c", "size": 2, "harm_tier": 2, "benefit_tier": 1},
        {"name": "d", "size": 0, "harm_tier": 3, "benefit_tier": 0},
    ]
    for group in groups:
        for exponent in [1.0, 1.5, 2.0]:
            vectors["group_impact"].append(
                {"group": group, "exponent": exponent,
                 "expected": decision_engine.calculate_group_impact(group, exponent)}
            )

    scenarios = [
        (0.1, groups[:2], 2.0, 1.5),
        (0.9, groups, 2.0, 1.5),
        (0.9, [groups[1], groups[0]], 0.5, 1.5),
        (0.9, [groups[1]], 2.0, 1.5),
        (0.9, [groups[0]], 2.0, 1.5),
        (0.6, [groups[2], groups[0]], 2.0, 2.0),
        (0.6, [groups[2], groups[3]], 2.0, 1.5),
        (0.87, [], 2.0, 1.5),
    ]
    for confidence, group_set, threshold, exponent in scenarios:
        vectors["score_decision"].append(
            {
                "confidence": confidence,
                "groups": group_set,
                "threshold": threshold,
                "exponent": exponent,
                "expected": decision_engine.score_decision(
                    confidence, group_set, threshold, exponent
                ),
            }
        )
    return vectors


def gen_emotion(rng):
    vectors = {"analyze": [], "outward": [], "style": []}

    long_previous = (
        "I was thinking about setting up some security cameras around my home and "
        "wanted to discuss the best placement options for covering all entry points."
    )
    currents = [
        "I was thinking about setting up some security cameras around my home.",
        "I am really worried there might be someone watching my house.",
        "Someone is trying to break in right now what do I do",
        "wait",
        "stop",
        "actually",
        "the weather is nice today",
        "I feel stuck on this paperwork",
        "there is a fire in the kitchen",
        "they are following me down the street",
        "I need help choosing a bus route",
        "currently the door will not lock",
        "hurry",
        "hello there friend",
    ]
    for current in currents:
        for previous in [None, long_previous, "short one"]:
            vectors["analyze"].append(
                {
                    "current": current,
                    "previous": previous,
                    "expected": emotional_framework.analyze_user_state(current, previous),
                }
            )
            vectors["outward"].append(
                {
                    "current": current,
                    "previous": previous,
                    "expected": emotional_framework.run_outward_channel(current, previous),
                }
            )

    for _ in range(40):
        words = [rng.choice(emotional_framework.STRESSED_INDICATORS
                            + emotional_framework.CRISIS_INDICATORS
                            + ["plain", "ordinary", "calm", "thinking"])]
        sentence = " ".join(words + ["about", "the", "situation", "today"])
        vectors["analyze"].append(
            {
                "current": sentence,
                "previous": None,
                "expected": emotional_framework.analyze_user_state(sentence, None),
            }
        )

    for state in ["stable", "stressed", "crisis", "unknown"]:
        vectors["style"].append(
            {"state": state, "expected": emotional_framework.get_communication_style(state)}
        )
    return vectors


def gen_cases():
    vectors = {"similarity": [], "db_script": []}

    stored = {
        "recurring_preparation_gap": True,
        "solution_type": "departure_point_reminder",
        "user_receptive_to_reminders": True,
        "timing": "pre_departure",
    }
    queries = [
        dict(stored),
        {"recurring_preparation_gap": True, "solution_type": "departure_point_reminder"},
        {"recurring_preparation_gap": False, "solution_type": "departure_point_reminder",
         "user_receptive_to_reminders": True, "timing": "pre_departure"},
        {},
        {"unrelated": 1},
        {"timing": "pre_departure", "solution_type": "departure_point_reminder",
         "user_receptive_to_reminders": True, "recurring_preparation_gap": True, "extra": "x"},
    ]
    for query in queries:
        vectors["similarity"].append(
            {"stored": stored, "query": query,
             "expected": case_library.calculate_similarity(stored, query)}
        )
    vectors["similarity"].append(
        {"stored": {}, "query": {"a": 1},
         "expected": case_library.calculate_similarity({}, {"a": 1})}
    )

    tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
    tmp.close()
    case_library.DB_PATH = tmp.name
    conn = case_library.initialize_database()

    def record(op, args, result):
        vectors["db_script"].append({"op": op, "args": args, "expected": mask(result)})

    factors = dict(stored)
    r = case_library.store_case(
        conn,
        description="User repeatedly forgets writing implement before meetings",
        factors=factors,
        solution="Remind user to bring pen as they prepare to leave for meeting",
        outcome_score=0.9,
    )
    record("store_case", {
        "description": "User repeatedly forgets writing implement before meetings",
        "factors": factors,
        "solution": "Remind user to bring pen as they prepare to leave for meeting",
        "outcome_score": 0.9,
    }, r)

    r = case_library.store_case(
        conn,
        description="User repeatedly forgets keys before leaving home",
        factors=factors,
        solution="Remind user to check for keys as they prepare to leave",
        outcome_score=0.85,
    )
    record("store_case", {
        "description": "User repeatedly forgets keys before leaving home",
        "factors": factors,
        "solution": "Remind user to check for keys as they prepare to leave",
        "outcome_score": 0.85,
    }, r)

    partial = {"recurring_preparation_gap": True, "solution_type": "departure_point_reminder"}
    r = case_library.store_case(
        conn,
        description="User forgets medication at departure",
        factors=partial,
        solution="Remind user about medication at the door",
        outcome_score=0.7,
    )
    record("store_case", {
        "description": "User forgets medication at departure",
        "factors": partial,
        "solution": "Remind user about medication at the door",
        "outcome_score": 0.7,
    }, r)

    for query, min_sim, max_res in [
        (factors, 0.5, 5),
        (partial, 0.5, 5),
        (partial, 0.9, 2),
        ({"unrelated": 1}, 0.5, 5),
    ]:
        r = case_library.find_similar_cases(conn, query, min_sim, max_res)
        record("find_similar_cases",
               {"query": query, "min_similarity": min_sim, "max_results": max_res}, r)

    r = case_library.update_outcome(conn, 1, 0.95)
    record("update_outcome", {"case_id": 1, "new_result": 0.95}, r)
    r = case_library.update_outcome(conn, 2, 0.1)
    record("update_outcome", {"case_id": 2, "new_result": 0.1}, r)

    injection = "'; DROP TABLE cases; --"
    for _ in range(3):
        r = case_library.store_case(conn, injection, {}, "malicious", 1.0)
        record("store_case", {
            "description": injection, "factors": {}, "solution": "malicious",
            "outcome_score": 1.0,
        }, r)

    r = case_library.check_injection_history(conn)
    record("check_injection_history", {"window_minutes": 30}, r)

    r = case_library.get_security_log(conn)
    record("get_security_log", {"unreviewed_only": True}, r)

    r = case_library.display_security_report(conn)
    record("display_security_report", {}, r)

    r = case_library.display_security_report(conn)
    record("display_security_report", {}, r)

    for _ in range(4):
        r = case_library.update_outcome(conn, 3, 0.0)
        record("update_outcome", {"case_id": 3, "new_result": 0.0}, r)

    r = case_library.find_similar_cases(conn, partial, 0.5, 5)
    record("find_similar_cases", {"query": partial, "min_similarity": 0.5, "max_results": 5}, r)

    r = case_library.update_outcome(conn, 999, 0.5)
    record("update_outcome", {"case_id": 999, "new_result": 0.5}, r)

    r = case_library.store_case(conn, injection, {}, "malicious", 1.0)
    record("store_case", {
        "description": injection, "factors": {}, "solution": "malicious",
        "outcome_score": 1.0,
    }, r)

    r = case_library.get_security_log(conn)
    record("get_security_log", {"unreviewed_only": True}, r)

    r = case_library.mark_security_log_reviewed(conn, [4])
    record("mark_security_log_reviewed", {"entry_ids": [4]}, r)

    r = case_library.get_security_log(conn, unreviewed_only=False)
    record("get_security_log", {"unreviewed_only": False}, r)

    conn.close()
    os.unlink(tmp.name)
    return vectors


def main():
    rng = random.Random(20261009)
    os.makedirs(OUT_DIR, exist_ok=True)
    payloads = {
        "decision.json": gen_decision(rng),
        "emotion.json": gen_emotion(rng),
        "cases.json": gen_cases(),
    }
    for name, payload in payloads.items():
        path = os.path.join(OUT_DIR, name)
        with open(path, "w") as handle:
            json.dump(payload, handle, indent=2)
        print(f"wrote {path}")


if __name__ == "__main__":
    main()
