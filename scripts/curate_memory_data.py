#!/usr/bin/env python3
"""
Curate and extract training data from OpenClaw memory.
PRIORITY: Clean data before generating synthetic pairs.
"""

import json
import re
from pathlib import Path

# Paths
MEMORY_MD = Path("C:/Users/belal/clawd/agents/ventas/MEMORY.md")
USER_MD = Path("C:/Users/belal/clawd/agents/ventas/USER.md")
SOUL_MD = Path("C:/Users/belal/clawd/agents/ventas/SOUL.md")
DAILY_DIR = Path("C:/Users/belal/clawd/agents/ventas/memory")
OUTPUT = Path("E:/datasetsDrive/training/curated_raw.jsonl")

# Sensitive patterns to REMOVE
SENSITIVE_PATTERNS = [
    (r'API_KEY[^:\n]{1,50}[a-zA-Z0-9\-_]{20,}', '[API_KEY_REDACTED]'),
    (r'tvly-dev-[a-zA-Z0-9]{20,}', '[TAVILY_KEY_REDACTED]'),
    (r'BSA[a-zA-Z0-9]{20,}', '[BRAVE_KEY_REDACTED]'),
    (r'password["\s:]{1,3}[^\s,]{8,}', '[PASSWORD_REDACTED]'),
    (r'secret["\s:]{1,3}[^\s,]{8,}', '[SECRET_REDACTED]'),
    (r'Bearer\s+[a-zA-Z0-9\-_]{20,}', '[TOKEN_REDACTED]'),
]

def sanitize(text: str) -> str:
    """Remove sensitive data from text."""
    for pattern, replacement in SENSITIVE_PATTERNS:
        text = re.sub(pattern, replacement, text)
    return text

def extract_from_markdown_file(filepath: Path) -> list:
    """Extract structured knowledge from a markdown file."""
    entries = []

    if not filepath.exists():
        print(f"  File not found: {filepath}")
        return entries

    content = filepath.read_text(encoding='utf-8')
    content = sanitize(content)

    # Extract key-value pairs (Pattern: "Key: Value" or "**Key:** Value")
    lines = content.split('\n')

    current_section = "general"
    for line in lines:
        line = line.strip()

        # Headers become sections
        if line.startswith('##'):
            current_section = line.replace('#', '').strip()
            continue

        # Key-value extraction
        if ': ' in line and len(line) > 10:
            # Skip very long lines (probably code blocks)
            if line.startswith('`') or line.startswith('    '):
                continue

            parts = line.split(': ', 1)
            if len(parts) == 2:
                key, value = parts
                key = key.strip().strip('*')
                value = value.strip().strip('*')

                if len(value) > 15 and len(key) > 2:
                    entries.append({
                        "source": filepath.name,
                        "section": current_section,
                        "key": key,
                        "value": value,
                        "raw_line": line
                    })

    return entries

def extract_daily_memory(filepath: Path) -> dict:
    """Extract summary from daily memory file."""
    if not filepath.exists():
        return {}

    content = filepath.read_text(encoding='utf-8')
    content = sanitize(content)

    # Extract key events, decisions, learnings
    events = []
    decisions = []

    lines = content.split('\n')
    for line in lines:
        line = line.strip()

        # Skip short lines
        if len(line) < 20:
            continue

        # Decision patterns
        if any(kw in line.lower() for kw in ['decided', 'decision', 'eligio', 'scoglie', 'decided']):
            decisions.append(line)

        # Event patterns
        if any(kw in line.lower() for kw in ['created', 'implemented', 'finished', 'completed', 'actualizo']):
            events.append(line)

    return {
        "date": filepath.stem,
        "events": events[:5],  # Limit
        "decisions": decisions[:5],
        "excerpt": content[:500]  # First 500 chars
    }

def generate_training_pairs(entries: list, daily_memories: list) -> list:
    """Generate Q&A pairs from curated entries."""
    pairs = []

    # From MEMORY.md entries
    for entry in entries:
        source = entry.get('source', 'unknown')
        section = entry.get('section', 'general')
        key = entry.get('key', '')
        value = entry.get('value', '')

        # Skip if too short or contains sensitive markers
        if len(value) < 20:
            continue
        if any(marker in value for marker in ['[REDACTED]', 'API_KEY', 'password']):
            continue

        # Generate appropriate Q&A based on key pattern
        key_lower = key.lower()

        if any(kw in key_lower for kw in ['quien', 'quiÃ©n', 'who', 'name', 'nombre']):
            pairs.append({
                "instruction": f"QuiÃ©n es {key}?",
                "response": value,
                "source": source,
                "category": "person_knowledge"
            })
        elif any(kw in key_lower for kw in ['precio', 'price', 'costo']):
            pairs.append({
                "instruction": f"CuÃ¡l es el precio de {key}?",
                "response": value,
                "source": source,
                "category": "pricing"
            })
        elif any(kw in key_lower for kw in ['url', 'link', 'enlace']):
            # Skip URLs - not good for training
            continue
        elif any(kw in key_lower for kw in ['project', 'proyecto', 'product']):
            pairs.append({
                "instruction": f"QuÃ© es {key}?",
                "response": value,
                "source": source,
                "category": "entity_knowledge"
            })
        elif any(kw in key_lower for kw in ['status', 'estado']):
            pairs.append({
                "instruction": f"Cual es el estado de {key}?",
                "response": value,
                "source": source,
                "category": "status"
            })
        else:
            # Generic Q&A
            if len(value) > 40:
                pairs.append({
                    "instruction": f"QuÃ© sabes sobre {key}?",
                    "response": value,
                    "source": source,
                    "category": section.lower().replace(' ', '_')
                })

    # From daily memories - extract operations/patterns
    for dm in daily_memories:
        date = dm.get('date', '')

        # Generate operational knowledge pairs
        for event in dm.get('events', []):
            if len(event) > 50:
                pairs.append({
                    "instruction": f"QuÃ© pasÃ³ el {date} con respecto a operaciones?",
                    "response": event,
                    "source": f"memory/{date}.md",
                    "category": "operation_event"
                })

        for decision in dm.get('decisions', []):
            if len(decision) > 30:
                pairs.append({
                    "instruction": f"QuÃ© decisiÃ³n se tomÃ³ el {date}?",
                    "response": decision,
                    "source": f"memory/{date}.md",
                    "category": "decision"
                })

    return pairs

def main():
    print("=" * 60)
    print("STEP 1: CURATE MEMORY DATA")
    print("=" * 60)

    all_entries = []
    all_daily = []

    # Extract from core files
    print("\n[1] Extracting from MEMORY.md...")
    entries = extract_from_markdown_file(MEMORY_MD)
    print(f"  Found {len(entries)} entries")
    all_entries.extend(entries)

    print("\n[2] Extracting from USER.md...")
    entries = extract_from_markdown_file(USER_MD)
    print(f"  Found {len(entries)} entries")
    all_entries.extend(entries)

    print("\n[3] Extracting from SOUL.md...")
    entries = extract_from_markdown_file(SOUL_MD)
    print(f"  Found {len(entries)} entries")
    all_entries.extend(entries)

    print("\n[4] Extracting daily memories (last 30 days)...")
    if DAILY_DIR.exists():
        daily_files = sorted(DAILY_DIR.glob("2026-*.md"), reverse=True)[:30]
        for df in daily_files:
            if df.name == '.dreams':
                continue
            dm = extract_daily_memory(df)
            if dm.get('events') or dm.get('decisions'):
                all_daily.append(dm)
        print(f"  Processed {len(all_daily)} daily memories")

    print("\n[5] Generating training pairs...")
    pairs = generate_training_pairs(all_entries, all_daily)
    print(f"  Generated {len(pairs)} pairs")

    # Deduplicate
    seen = set()
    unique_pairs = []
    for p in pairs:
        key = p['instruction'].lower().strip()
        if key not in seen and len(key) > 10:
            seen.add(key)
            unique_pairs.append(p)

    print(f"  After dedup: {len(unique_pairs)} pairs")

    # Save
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT, 'w', encoding='utf-8') as f:
        for p in unique_pairs:
            f.write(json.dumps(p, ensure_ascii=False) + '\n')

    print(f"\nâœ… Curated data saved to: {OUTPUT}")

    # Show category distribution
    cats = {}
    for p in unique_pairs:
        cat = p.get('category', 'unknown')
        cats[cat] = cats.get(cat, 0) + 1

    print("\nCategory distribution:")
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")

    return unique_pairs

if __name__ == "__main__":
    main()

