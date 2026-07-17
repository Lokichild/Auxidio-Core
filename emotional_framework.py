# Copyright (c) 2026 Jerry James Stephens / Bound Wolf Technologies
# All rights reserved. No part of this code may be copied, modified,
# or distributed without explicit written permission from the author.

# Auxidio Emotional Framework
# Inward Facing Channel - Concern for Self

import psutil
import shutil

# Warning thresholds
THRESHOLDS = {
    "cpu_temp": 80.0,      # Celsius
    "memory": 85.0,         # Percentage
    "storage": 90.0,        # Percentage
}

# Warning priority levels
IMMEDIATE = "immediate"
CONVENIENT = "convenient"

def check_cpu_temperature():
    """
    Monitors CPU temperature and returns warning if threshold exceeded.
    """
    try:
        temps = psutil.sensors_temperatures()
        if not temps:
            return None
            
        # Find the highest CPU temperature reading
        cpu_temp = None
        for name, entries in temps.items():
            for entry in entries:
                if cpu_temp is None or entry.current > cpu_temp:
                    cpu_temp = entry.current
        
        if cpu_temp and cpu_temp >= THRESHOLDS["cpu_temp"]:
            return {
                "priority": IMMEDIATE,
                "system": "cpu_temperature",
                "current_value": round(cpu_temp, 1),
                "threshold": THRESHOLDS["cpu_temp"],
                "warning": f"I am overheating. My current temperature is {round(cpu_temp, 1)}°C, which is above my safe operating limit of {THRESHOLDS['cpu_temp']}°C.",
                "suggestion": "Please move me to a cooler location, ensure my fan is unobstructed, and reduce the number of active tasks. I may need to rest for a few minutes."
            }
    except Exception as e:
        return None
    return None


def check_memory():
    """
    Monitors RAM usage and returns warning if threshold exceeded.
    """
    try:
        memory = psutil.virtual_memory()
        usage_percent = memory.percent
        
        if usage_percent >= THRESHOLDS["memory"]:
            return {
                "priority": CONVENIENT,
                "system": "memory",
                "current_value": round(usage_percent, 1),
                "threshold": THRESHOLDS["memory"],
                "warning": f"My memory is running low. I am currently using {round(usage_percent, 1)}% of available memory.",
                "suggestion": "I would benefit from closing any unused applications or processes. If this continues, consider restarting me when convenient."
            }
    except Exception as e:
        return None
    return None


def check_storage():
    """
    Monitors storage usage across both SSDs and returns warning if threshold exceeded.
    """
    warnings = []
    drives = ["/", "/mnt/ssd2"] # Adjust paths to match actual SSD mount points
    
    for drive in drives:
        try:
            usage = shutil.disk_usage(drive)
            percent_used = (usage.used / usage.total) * 100
            
            if percent_used >= THRESHOLDS["storage"]:
                warnings.append({
                    "priority": CONVENIENT,
                    "system": f"storage_{drive}",
                    "current_value": round(percent_used, 1),
                    "threshold": THRESHOLDS["storage"],
                    "warning": f"My storage at {drive} is running low. I am currently using {round(percent_used, 1)}% of available space.",
                    "suggestion": "Consider clearing older case library entries or expanding my storage capacity. I may have difficulty learning new solutions if this continues."
                })
        except Exception:
            continue
    
    return warnings if warnings else None


def run_inward_channel():
    """
    Runs all inward facing checks and returns categorized warnings.
    Returns two lists: immediate warnings and convenient warnings.
    """
    immediate_warnings = []
    convenient_warnings = []
    
    # Check each system
    temp_warning = check_cpu_temperature()
    if temp_warning:
        immediate_warnings.append(temp_warning)
    
    memory_warning = check_memory()
    if memory_warning:
        convenient_warnings.append(memory_warning)
    
    storage_warnings = check_storage()
    if storage_warnings:
        convenient_warnings.extend(storage_warnings)
    
    return immediate_warnings, convenient_warnings


def report_inward_status():
    """
    Generates a human readable status report from inward channel checks.
    Returns a dict with immediate and convenient warnings, and an all_clear flag.
    """
    immediate, convenient = run_inward_channel()
    
    all_clear = len(immediate) == 0 and len(convenient) == 0
    
    return {
        "all_clear": all_clear,
        "immediate_warnings": immediate,
        "convenient_warnings": convenient
    }


# Startup check including battery prompt
def startup_check():
    """
    Runs at system startup. Checks all systems and prompts user
    to confirm adequate battery charge before beginning session.
    """
    print("Auxidio is initializing. Running startup checks...")
    print()
    
    status = report_inward_status()
    
    if status["all_clear"]:
        print("All internal systems are operating normally.")
    else:
        if status["immediate_warnings"]:
            print("IMMEDIATE ATTENTION REQUIRED:")
            for warning in status["immediate_warnings"]:
                print(f"  WARNING: {warning['warning']}")
                print(f"  SUGGESTION: {warning['suggestion']}")
                print()
        
        if status["convenient_warnings"]:
            print("Please address when convenient:")
            for warning in status["convenient_warnings"]:
                print(f"  Note: {warning['warning']}")
                print(f"  Suggestion: {warning['suggestion']}")
                print()
    
    # Battery prompt regardless of other status
    print("I am unable to directly monitor battery level.")
    print("Before we begin, please confirm: do you have sufficient")
    print("power available for this session? (yes/no)")
    battery_confirmed = input("> ").strip().lower()
    
    if battery_confirmed != "yes":
        print("Please ensure adequate power before we begin.")
        print("I will wait until you are ready.")
        return False
    
    print()
    print("Startup checks complete. I am ready.")
    return True


# Outward Facing Channel - Concern for Others

# User state tiers
STABLE = "stable"
STRESSED = "stressed"
CRISIS = "crisis"

# Keywords and patterns that suggest elevated states
STRESSED_INDICATORS = [
    "worried", "concerned", "anxious", "nervous", "scared",
    "problem", "trouble", "difficult", "struggling", "help",
    "upset", "frustrated", "confused", "lost", "stuck"
]

CRISIS_INDICATORS = [
    "right now", "happening now", "currently", "emergency",
    "urgent", "immediately", "someone is", "they are",
    "breaking in", "attack", "danger", "hurt", "bleeding",
    "fire", "threatening", "following me", "inside my"
]

TONE_SHIFT_INDICATORS = [
    "wait", "stop", "hold on", "never mind", "forget that",
    "actually", "quickly", "hurry", "fast", "now"
]

def analyze_user_state(current_input, previous_input=None):
    """
    Infers user emotional state from conversation content and tone.
    Detects sudden tone shifts if previous input is provided.
    
    current_input: string, what the user just said
    previous_input: string, what the user said before, if available
    
    Returns state tier and reasoning.
    """
    current_lower = current_input.lower()
    
    # Check for sudden tone shift first
    if previous_input:
        previous_lower = previous_input.lower()
        tone_shift_detected = any(
            indicator in current_lower 
            for indicator in TONE_SHIFT_INDICATORS
        )
        
        # Check for shift from longer to very short input
        length_shift = (
            len(previous_input.split()) > 10 and 
            len(current_input.split()) < 5
        )
        
        if tone_shift_detected and length_shift:
            return {
                "state": CRISIS,
                "reasoning": "Multiple tone shift indicators detected simultaneously.",
                "clarification_needed": True,
                "clarification_question": "What is going on? Are you okay?"
            }
        elif tone_shift_detected or length_shift:
            return {
                "state": CRISIS,
                "reasoning": "Sudden change in communication pattern detected.",
                "clarification_needed": True,
                "clarification_question": "Are you dealing with something right now?"
            }
    
    # Check for crisis indicators
    crisis_matches = [
        indicator for indicator in CRISIS_INDICATORS 
        if indicator in current_lower
    ]
    if crisis_matches:
        return {
            "state": CRISIS,
            "reasoning": f"Immediate situation language detected.",
            "clarification_needed": False,
            "clarification_question": None
        }
    
    # Check for stressed indicators
    stressed_matches = [
        indicator for indicator in STRESSED_INDICATORS 
        if indicator in current_lower
    ]
    if stressed_matches:
        return {
            "state": STRESSED,
            "reasoning": f"Elevated concern language detected.",
            "clarification_needed": False,
            "clarification_question": None
        }
    
    # Default to stable
    return {
        "state": STABLE,
        "reasoning": "No elevated state indicators detected.",
        "clarification_needed": False,
        "clarification_question": None
    }


def get_communication_style(state):
    """
    Returns communication guidelines based on detected user state.
    These guide how responses are packaged, not what decisions are made.
    """
    if state == CRISIS:
        return {
            "state": CRISIS,
            "tone": "Direct and decisive.",
            "response_length": "Brief. Essential information only.",
            "question_style": "Single, plain, immediate. Example: 'Are you safe right now?'",
            "elaboration": "None unless explicitly requested.",
            "pacing": "Immediate response priority."
        }
    elif state == STRESSED:
        return {
            "state": STRESSED,
            "tone": "Calm and confident.",
            "response_length": "Concise. Clear and complete but not lengthy.",
            "question_style": "Direct and purposeful. Brief explanation of why if needed.",
            "elaboration": "Available if requested, not offered automatically.",
            "pacing": "Steady and unhurried."
        }
    else:
        return {
            "state": STABLE,
            "tone": "Conversational and thorough.",
            "response_length": "As needed. Full explanations offered naturally.",
            "question_style": "Open and exploratory.",
            "elaboration": "Offered freely.",
            "pacing": "Natural conversation flow."
        }


def run_outward_channel(current_input, previous_input=None):
    """
    Main outward channel function. Analyzes user state and
    returns appropriate communication style guidance.
    """
    state_analysis = analyze_user_state(current_input, previous_input)
    communication_style = get_communication_style(state_analysis["state"])
    
    return {
        "user_state": state_analysis["state"],
        "reasoning": state_analysis["reasoning"],
        "clarification_needed": state_analysis["clarification_needed"],
        "clarification_question": state_analysis["clarification_question"],
        "communication_style": communication_style
    }

if __name__ == "__main__":
    # Test inward channel
    print("=== INWARD CHANNEL ===")
    startup_check()
    print()
    
    # Test outward channel - stable state
    print("=== OUTWARD CHANNEL TESTS ===")
    print()
    
    print("Test 1: Stable state")
    result = run_outward_channel(
        "I was thinking about setting up some security cameras around my home."
    )
    print(f"State: {result['user_state']}")
    print(f"Reasoning: {result['reasoning']}")
    print(f"Tone: {result['communication_style']['tone']}")
    print()
    
    print("Test 2: Stressed state")
    result = run_outward_channel(
        "I am really worried there might be someone watching my house."
    )
    print(f"State: {result['user_state']}")
    print(f"Reasoning: {result['reasoning']}")
    print(f"Tone: {result['communication_style']['tone']}")
    print()
    
    print("Test 3: Crisis state")
    result = run_outward_channel(
        "Someone is trying to break in right now what do I do"
    )
    print(f"State: {result['user_state']}")
    print(f"Reasoning: {result['reasoning']}")
    print(f"Tone: {result['communication_style']['tone']}")
    print()
    
    print("Test 4: Tone shift detection")
    result = run_outward_channel(
        "wait",
        previous_input="I was thinking about setting up some security cameras around my home and wanted to discuss the best placement options for covering all entry points."
    )
    print(f"State: {result['user_state']}")
    print(f"Reasoning: {result['reasoning']}")
    print(f"Clarification needed: {result['clarification_needed']}")
    print(f"Clarification question: {result['clarification_question']}")
