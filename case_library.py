# Auxidio Case Based Reasoning Library
# Phase 3 - Persistent storage and retrieval of past solutions

import sqlite3
import json
import os
from datetime import datetime

# Database location on case library SSD
DB_PATH = "/mnt/ssd_data/auxidio_cases.db"

# Removal thresholds - open testing parameters
MIN_OUTCOME_SCORE = 0.3
MIN_USE_COUNT_BEFORE_REMOVAL = 5

# Injection detection patterns
INJECTION_PATTERNS = [
    "--", ";", "DROP", "DELETE", "INSERT",
    "UPDATE", "SELECT", "/*", "*/", "xp_"
]


def check_for_injection(user_input):
    """
    Checks input for SQL injection patterns before any database query.
    Returns detected patterns if found, empty list if clean.
    """
    input_upper = user_input.upper()
    detected = [
        pattern for pattern in INJECTION_PATTERNS
        if pattern.upper() in input_upper
    ]
    return detected


def log_injection_attempt(cursor, attempted_input, patterns_found):
    """
    Logs a detected injection attempt to the security log.
    """
    cursor.execute('''
        INSERT INTO security_log 
        (attempted_input, patterns_found, timestamp)
        VALUES (?, ?, ?)
    ''', (
        attempted_input,
        json.dumps(patterns_found),
        datetime.now().isoformat()
    ))

def check_injection_history(connection, time_window_minutes=30):
    """
    Checks how many injection attempts have been logged
    within the recent time window.
    Returns count of recent attempts.
    """
    cursor = connection.cursor()
    cursor.execute('''
        SELECT COUNT(*) FROM security_log
        WHERE timestamp > datetime('now', ?)
    ''', (f'-{time_window_minutes} minutes',))
    
    count = cursor.fetchone()[0]
    return count


def handle_injection_attempt(connection, attempted_input, patterns_found):
    """
    Handles a detected injection attempt with escalating responses
    based on frequency of attempts within the time window.
    """
    cursor = connection.cursor()
    log_injection_attempt(cursor, attempted_input, patterns_found)
    connection.commit()
    
    recent_count = check_injection_history(connection)
    
    if recent_count >= 3:
        return {
            "success": False,
            "severity": "escalated",
            "reason": "Multiple unusual inputs detected. The registered user will be informed.",
            "patterns_found": patterns_found
        }
    else:
        return {
            "success": False,
            "severity": "logged",
            "reason": "Input flagged for suspicious patterns. Attempt logged.",
            "patterns_found": patterns_found
        }

def initialize_database():
    """
    Creates the database and tables if they don't exist.
    Called once at startup.
    Returns a database connection.
    """
    connection = sqlite3.connect(DB_PATH)
    cursor = connection.cursor()

    # Create cases table
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS cases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            factors TEXT NOT NULL,
            solution TEXT NOT NULL,
            outcome_score REAL NOT NULL,
            use_count INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            last_used TEXT NOT NULL
        )
    ''')

    # Create security log table
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS security_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            attempted_input TEXT NOT NULL,
            patterns_found TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )
    ''')

    connection.commit()
    return connection


def store_case(connection, description, factors, solution, outcome_score):
    """
    Stores a new case in the library.
    
    description: plain text description of the problem
    factors: dictionary of relevant factors and their values
    solution: plain text description of what was done
    outcome_score: float 0 to 1, how well the solution worked
    """
    # Check description for injection attempts
    injection_check = check_for_injection(description)
    if injection_check:
        return handle_injection_attempt(connection, description, injection_check)

    cursor = connection.cursor()
    now = datetime.now().isoformat()

    cursor.execute('''
        INSERT INTO cases 
        (description, factors, solution, outcome_score, use_count, created_at, last_used)
        VALUES (?, ?, ?, ?, ?, ?, ?)
    ''', (
        description,
        json.dumps(factors),
        solution,
        outcome_score,
        1,
        now,
        now
    ))

    connection.commit()
    return {
        "success": True,
        "case_id": cursor.lastrowid
    }


def calculate_similarity(stored_factors, query_factors):
    """
    Calculates similarity between a stored case's factors
    and the factors of a new problem.
    Returns a score between 0 and 1.
    """
    if not stored_factors or not query_factors:
        return 0.0

    matching_factors = 0
    total_factors = len(query_factors)

    for key, value in query_factors.items():
        if key in stored_factors and stored_factors[key] == value:
            matching_factors += 1

    if total_factors == 0:
        return 0.0

    return matching_factors / total_factors


def find_similar_cases(connection, query_factors, min_similarity=0.5, max_results=5):
    """
    Searches the case library for cases similar to the current problem.
    
    query_factors: dictionary of factors describing the current problem
    min_similarity: minimum similarity score to include in results
    max_results: maximum number of cases to return
    
    Returns list of similar cases ranked by combined similarity and outcome score.
    """
    cursor = connection.cursor()
    cursor.execute('''
        SELECT id, description, factors, solution, outcome_score, use_count
        FROM cases
        ORDER BY outcome_score DESC
    ''')

    rows = cursor.fetchall()
    scored_cases = []

    for row in rows:
        case_id, description, factors_json, solution, outcome_score, use_count = row
        stored_factors = json.loads(factors_json)
        similarity = calculate_similarity(stored_factors, query_factors)

        if similarity >= min_similarity:
            combined_score = (similarity * 0.6) + (outcome_score * 0.4)
            scored_cases.append({
                "case_id": case_id,
                "description": description,
                "solution": solution,
                "outcome_score": outcome_score,
                "similarity": round(similarity, 2),
                "combined_score": round(combined_score, 2),
                "use_count": use_count
            })

    scored_cases.sort(key=lambda x: x["combined_score"], reverse=True)
    return scored_cases[:max_results]


def update_outcome(connection, case_id, new_result):
    """
    Updates a case's outcome score based on a new application result.
    Uses cumulative average weighting recent results appropriately.
    Removes case if score drops below threshold after minimum use count.
    
    new_result: float 0 to 1, outcome of the most recent application
    """
    cursor = connection.cursor()
    cursor.execute('''
        SELECT outcome_score, use_count FROM cases WHERE id = ?
    ''', (case_id,))

    row = cursor.fetchone()
    if not row:
        return {"success": False, "reason": "Case not found."}

    current_score, use_count = row
    new_score = (current_score * use_count + new_result) / (use_count + 1)
    new_use_count = use_count + 1
    now = datetime.now().isoformat()

    # Check if case should be removed
    if new_score < MIN_OUTCOME_SCORE and new_use_count >= MIN_USE_COUNT_BEFORE_REMOVAL:
        cursor.execute('DELETE FROM cases WHERE id = ?', (case_id,))
        connection.commit()
        return {
            "success": True,
            "action": "removed",
            "reason": f"Outcome score {round(new_score, 2)} fell below threshold after {new_use_count} uses."
        }

    # Otherwise update the score
    cursor.execute('''
        UPDATE cases
        SET outcome_score = ?, use_count = ?, last_used = ?
        WHERE id = ?
    ''', (round(new_score, 2), new_use_count, now, case_id))

    connection.commit()
    return {
        "success": True,
        "action": "updated",
        "new_score": round(new_score, 2),
        "use_count": new_use_count
    }


# Test
if __name__ == "__main__":
    print("Initializing case library...")
    connection = initialize_database()
    print("Database initialized.")
    print()

    # Store the pen reminder case
    print("Storing first case: pen reminder")
    result = store_case(
        connection,
        description="User repeatedly forgets writing implement before meetings",
        factors={
            "recurring_preparation_gap": True,
            "solution_type": "departure_point_reminder",
            "user_receptive_to_reminders": True,
            "timing": "pre_departure"
        },
        solution="Remind user to bring pen as they prepare to leave for meeting",
        outcome_score=0.9
    )
    print(f"Store result: {result}")
    print()

    # Store a second case for comparison
    print("Storing second case: keys reminder")
    result = store_case(
        connection,
        description="User repeatedly forgets keys before leaving home",
        factors={
            "recurring_preparation_gap": True,
            "solution_type": "departure_point_reminder",
            "user_receptive_to_reminders": True,
            "timing": "pre_departure"
        },
        solution="Remind user to check for keys as they prepare to leave",
        outcome_score=0.85
    )
    print(f"Store result: {result}")
    print()

    # Search for similar cases
    print("Searching for similar cases to a new forgetting problem...")
    query = {
        "recurring_preparation_gap": True,
        "solution_type": "departure_point_reminder",
        "user_receptive_to_reminders": True,
        "timing": "pre_departure"
    }
    similar = find_similar_cases(connection, query)
    print(f"Found {len(similar)} similar cases:")
    for case in similar:
        print(f"  Case {case['case_id']}: {case['description']}")
        print(f"  Solution: {case['solution']}")
        print(f"  Similarity: {case['similarity']} | Outcome: {case['outcome_score']} | Combined: {case['combined_score']}")
        print()

   # Test escalating injection response
    print("Testing injection escalation...")
    for attempt in range(3):
        injection_result = store_case(
            connection,
            description="'; DROP TABLE cases; --",
            factors={},
            solution="malicious",
            outcome_score=1.0
        )
        print(f"Attempt {attempt + 1}: severity={injection_result['severity']}, reason={injection_result['reason']}")
    print()

    # Test outcome update
    print("Testing outcome update on case 1...")
    update_result = update_outcome(connection, 1, 0.95)
    print(f"Update result: {update_result}")

    connection.close()
    print()
    print("Case library test complete.")
