FROM python:3.11-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY scripts/benchmarks /workspace/scripts/benchmarks

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
