# Copyright (c) 2026 Jerry James Stephens / Bound Wolf Technologies
# All rights reserved. No part of this code may be copied, modified,
# or distributed without explicit written permission from the author.

# Auxidio Master Program
# Phase 5 - First Integrated Test
# Connects all modules into a single conversational loop

import sys
import os
import time
import threading
import tempfile
import subprocess
import sounddevice as sd
import soundfile as sf
import numpy as np
import whisper
import ollama
import evdev
from evdev import InputDevice, categorize, ecodes

# Add project directory to path so modules can be found
sys.path.insert(0, '/mnt/ssd_dev/Auxidio Engine')

from decision_engine import evaluate_decision, get_confidence_tier
from emotional_framework import (
    run_inward_channel, run_outward_channel, 
    startup_check, IMMEDIATE
)
from identity import initialize_identity
from case_library import (
    initialize_database, find_similar_cases, 
    store_case, display_security_report
)

# Audio configuration
SAMPLE_RATE = 44100
CHANNELS = 1
AUDIO_DEVICE_INDEX = None  # Will be detected at startup
OUTPUT_DEVICE_INDEX = None  # Will be detected at startup

# Piper voice configuration
PIPER_VOICE = os.path.expanduser(
    "~/piper_voices/en_US-lessac-medium.onnx"
)

# Whisper model size
WHISPER_MODEL = "small"

# Recording state
is_recording = False
recording_data = []
recording_lock = threading.Lock()

# Conversation history for tone shift detection
previous_input = None


def detect_audio_devices():
    """
    Detects USB microphone and audio output device at startup.
    Returns input and output device indices.
    """
    devices = sd.query_devices()
    input_device = None
    output_device = None
    
    for i, device in enumerate(devices):
        device_name = device['name'].lower()
        if 'usb' in device_name and device['max_input_channels'] > 0:
            if input_device is None:
                input_device = i
        if device['max_output_channels'] > 0:
            if 'usb' in device_name or 'uac' in device_name:
                output_device = i
    
    return input_device, output_device


def speak(text):
    """
    Converts text to speech using Piper and plays through audio output.
    """
    try:
        with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as tmp:
            tmp_path = tmp.name
        
        subprocess.run([
            'bash', '-c',
            f'echo "{text}" | '
            f'/home/orangepi/whisper_env/bin/python3 -m piper '
            f'--model {PIPER_VOICE} '
            f'--output_file {tmp_path}'
        ], check=True, capture_output=True)
        
        # Play using plughw to handle sample rate conversion automatically
        subprocess.run([
            'aplay', '-D', 'plughw:1,0', tmp_path
        ], check=True, capture_output=True)
        
        os.unlink(tmp_path)
        
    except Exception as e:
        print(f"Speech output error: {e}")


def record_audio():
    global recording_data
    recording_data = []
    callback_count = 0
    
    def callback(indata, frames, time, status):
        nonlocal callback_count
        if status:
            print(f"[Debug callback status: {status}]")
        if is_recording:
            callback_count += 1
            with recording_lock:
                recording_data.append(indata.copy())
    
    with sd.InputStream(
        samplerate=SAMPLE_RATE,
        channels=CHANNELS,
        device=AUDIO_DEVICE_INDEX,
        callback=callback
    ):
        while is_recording:
            time.sleep(0.1)


def transcribe_audio():
    if not recording_data:
        print("[Debug: recording_data is empty]")
        return ""
    
    with recording_lock:
        audio_data = np.concatenate(recording_data, axis=0)
    
    audio_float = audio_data.flatten().astype(np.float32)
    
    # Normalize
    max_val = np.max(np.abs(audio_float))
    if max_val > 0:
        audio_float = audio_float / max_val
    
    # Save to temp file and transcribe from file
    # This matches the approach that worked in our earlier test
    tmp_path = '/tmp/auxidio_recording.wav'
    sf.write(tmp_path, audio_float, SAMPLE_RATE)
    
    model = whisper.load_model(WHISPER_MODEL)
    result = model.transcribe(tmp_path, fp16=False)

    return result['text'].strip()


def generate_response(
    user_input, user_state, similar_cases, decision_result, identity
):
    """
    Generates a natural language response using Ollama,
    calibrated to user emotional state and decision engine output.
    """
    case_context = ""
    if similar_cases:
        top_case = similar_cases[0]
        case_context = (
            f"A similar situation was handled successfully before: "
            f"{top_case['solution']} "
            f"(confidence: {top_case['outcome_score']})"
        )
    
    communication_style = user_state['communication_style']
    
    system_prompt = f"""You are {identity['user_name']}, a personal reasoning 
and decision-support companion. You help your user think through decisions 
clearly and honestly.

Current communication guidelines:
- Tone: {communication_style['tone']}
- Response length: {communication_style['response_length']}
- Elaboration: {communication_style['elaboration']}

Decision engine assessment:
- Confidence tier: {decision_result['tier']}
- Decision score: {decision_result['score']}

{case_context}

Respond directly to the user's input following the communication guidelines 
above. Do not explain your reasoning process. Just respond helpfully and 
naturally."""

    response = ollama.chat(
        model='gemma3:1b',
        messages=[
            {'role': 'system', 'content': system_prompt},
            {'role': 'user', 'content': user_input}
        ]
    )
    
    return response['message']['content']


def process_interaction(user_input, db_connection, identity):
    """
    Processes a complete interaction from transcribed input to spoken response.
    """
    global previous_input
    
    print(f"\nYou said: {user_input}")
    
    # Analyze user emotional state
    user_state = run_outward_channel(user_input, previous_input)
    print(f"User state: {user_state['user_state']}")
    
    # Handle clarification if needed
    if user_state['clarification_needed']:
        speak(user_state['clarification_question'])
        previous_input = user_input
        return
    
    # Search case library for similar past interactions
    query_factors = {
        "user_state": user_state['user_state'],
        "input_length": "short" if len(user_input.split()) < 10 else "long"
    }
    similar_cases = find_similar_cases(db_connection, query_factors)
    
    # Evaluate with decision engine
    decision_result = evaluate_decision(
        confidence_rating=0.8,
        net_collective_impact=0.5,
        net_individual_impact=0.7
    )
    
    # Generate and speak response
    response = generate_response(
        user_input, user_state, similar_cases, 
        decision_result, identity
    )
    
    print(f"Auxidio: {response}")
    speak(response)
    
    # Store interaction in case library
    store_case(
        db_connection,
        description=user_input,
        factors=query_factors,
        solution=response,
        outcome_score=0.8
    )
    
    previous_input = user_input


def main():
    global is_recording, AUDIO_DEVICE_INDEX, OUTPUT_DEVICE_INDEX
    
    print("=== Auxidio Starting ===")
    print()
    
    # Initialize all systems
    identity = initialize_identity()
    db_connection = initialize_database()
    
    # Check for security events
    security_report = display_security_report(db_connection)
    if security_report != "No new security events to report.":
        print(security_report)
    
    # Run startup check
    startup_check()
    
    # Detect audio devices
    print("Detecting audio devices...")
    AUDIO_DEVICE_INDEX, OUTPUT_DEVICE_INDEX = detect_audio_devices()
    AUDIO_DEVICE_INDEX = 0
    OUTPUT_DEVICE_INDEX = 1

    if AUDIO_DEVICE_INDEX is None:
        print("Warning: No USB microphone detected. Please check connection.")
    else:
        print(f"Microphone detected on device {AUDIO_DEVICE_INDEX}")
    
    if OUTPUT_DEVICE_INDEX is None:
        print("Warning: No USB audio output detected. Using system default.")
    else:
        print(f"Audio output detected on device {OUTPUT_DEVICE_INDEX}")
    
    print()
    print(f"I am {identity['user_name']}. Hold the mouse button to speak.")
    speak(f"I am {identity['user_name']}. Hold the mouse button to speak.")
    
    # Mouse button handler using evdev
    MOUSE_DEVICE = '/dev/input/by-id/usb-INSTANT_USB_GAMING_MOUSE-event-mouse'
    
    print("Listening for mouse input. Hold left button to speak.")
    
    try:
        device = InputDevice(MOUSE_DEVICE)
        recording_thread = None
        
        for event in device.read_loop():
            if event.type == ecodes.EV_KEY:
                key_event = categorize(event)
                
                # Left mouse button is BTN_LEFT
                if 'BTN_LEFT' in key_event.keycode:
                    if key_event.keystate == key_event.key_down:
                        if not is_recording:
                            is_recording = True
                            recording_thread = threading.Thread(
                                target=record_audio
                            )
                            recording_thread.start()
                            print("\n[Recording... release to send]")
                    
                    elif key_event.keystate == key_event.key_up:
                        if is_recording:
                            is_recording = False
                            if recording_thread:
                                recording_thread.join()
                            
                            print("[Processing...]")
                            user_input = transcribe_audio()
                            
                            if user_input:
                                immediate, convenient = run_inward_channel()
                                if immediate:
                                    for warning in immediate:
                                        print(f"SYSTEM: {warning['warning']}")
                                        speak(warning['warning'])
                                
                                process_interaction(
                                    user_input, db_connection, identity
                                )
                            else:
                                print("[No speech detected]")
    
    except KeyboardInterrupt:
        print("\nAuxidio shutting down.")
        db_connection.close()
    except PermissionError:
        print("Permission denied accessing mouse device.")
        print("Run: sudo chmod a+r /dev/input/by-id/usb-INSTANT_USB_GAMING_MOUSE-event-mouse")
        db_connection.close()


if __name__ == "__main__":
    main()
