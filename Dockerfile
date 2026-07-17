# Copyright (c) 2026 Jerry James Stephens / Bound Wolf Technologies
# All rights reserved. No part of this code may be copied, modified,
# or distributed without explicit written permission from the author.

# Auxidio Core Container
# Contains: decision engine, emotional framework, identity, case library

# Start from official Python 3.12 image on slim Debian base
FROM python:3.12-slim

# Set working directory inside container
WORKDIR /app

# Install required system packages
RUN apt-get update && apt-get install -y \
    python3-psutil \
    && rm -rf /var/lib/apt/lists/*

# Copy core module files into container
COPY decision_engine.py .
COPY emotional_framework.py .
COPY identity.py .
COPY case_library.py .

# Create mount point for persistent data volume
RUN mkdir -p /mnt/ssd_data

# Default command, placeholder until master program exists
CMD ["python3", "-c", "print('Auxidio core container running.')"]
