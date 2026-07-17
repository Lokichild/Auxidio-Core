# Copyright (c) 2026 Jerry James Stephens / Bound Wolf Technologies
# All rights reserved. No part of this code may be copied, modified,
# or distributed without explicit written permission from the author.

# Auxidio Identity Module
# Generates and maintains persistent device identity

import uuid
import os
import json

# Path to identity storage on case library SSD
IDENTITY_FILE = "/mnt/ssd_data/auxidio_identity.json"

def generate_identity(user_name):
    """
    Generates a new Auxidio identity on first startup.
    user_name: the human facing name chosen by the user
    Returns the complete identity dictionary.
    """
    identity = {
        "user_name": user_name,
        "device_id": str(uuid.uuid4()),
        "created_at": str(__import__('datetime').datetime.now())
    }
    return identity

def save_identity(identity):
    """
    Saves identity to persistent storage on case library SSD.
    """
    try:
        with open(IDENTITY_FILE, 'w') as f:
            json.dump(identity, f, indent=4)
        return True
    except Exception as e:
        print(f"Warning: Could not save identity file. {e}")
        return False

def load_identity():
    """
    Loads existing identity from persistent storage.
    Returns identity dict if found, None if not found.
    """
    try:
        if os.path.exists(IDENTITY_FILE):
            with open(IDENTITY_FILE, 'r') as f:
                return json.load(f)
    except Exception as e:
        print(f"Warning: Could not load identity file. {e}")
    return None

def initialize_identity():
    """
    Called at startup. Loads existing identity or creates
    a new one if this is the first boot.
    Returns the active identity.
    """
    identity = load_identity()
    
    if identity:
        print(f"Identity loaded. Hello, I am {identity['user_name']}.")
        print(f"Device ID: {identity['device_id']}")
        return identity
    
    # First boot, generate new identity
    print("No identity found. This appears to be my first startup.")
    print("What would you like to call me?")
    user_name = input("> ").strip()
    
    if not user_name:
        user_name = "Auxidio"
    
    identity = generate_identity(user_name)
    
    if save_identity(identity):
        print(f"Identity created. I am {identity['user_name']}.")
        print(f"My device ID is: {identity['device_id']}")
        print("This ID is permanent and will travel with me if my hardware changes.")
    
    return identity

if __name__ == "__main__":
    identity = initialize_identity()
