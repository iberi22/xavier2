#!/usr/bin/env python3
"""
Export and curate Xavier2 memory data for fine-tuning.
Creates instruction-response pairs from memory documents.
"""

import json
import sys
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional
import re

# Configuration
XAVIER2_URL = "http://127.0.0.1:8003"
XAVIER2_TOKEN = "dev-token"
OUTPUT_FILE = Path(__file__).parent.parent / "data" / "training" / "xavier2_training.jsonl"

# Categories for training data
CATEGORIES = {
    "business": ["swal", "company", "business", "product"],
    "client": ["client", "leonardo", "rodacenter", "tripro"],
    "technical": ["xavier2", "cortex", "openclaw", "system", "config"],
    "sales": ["sales", "rfi", "proposal", "pricing", "cotizacion"],
    "projects": ["project", "gestalt", "synapse", "manteniapp"],
    "reasoning": ["decision", "analysis", "compare", "why", "porque"],
    "qa": ["who", "what", "where", "when", "how", "cuanto", "cual"]
}

# Templates for instruction-response pairs
QA_TEMPLATES = {
    "who": "Quién es {entity}?",
    "what": "Qué es {topic}?",
    "pricing": "Cuál es el precio de {product} {tier}?",
    "project": "Qué es {project} y para qué sirve?",
    "company": "Cuéntame sobre {company}",
    "process": "Cómo funciona el proceso de {process}?",
    "relation": "Qué relación hay entre {a} y {b}?",
}

def query_xavier2(query: str, limit: int = 50) -> List[Dict]:
    """Query Xavier2 memory API."""
    import urllib.request
    
    data = json.dumps({"query": query, "limit": limit}).encode()
    req = urllib.request.Request(
        f"{XAVIER2_URL}/memory/query",
        data=data,
        headers={
            "Content-Type": "application/json",
            "X-Xavier2-Token": XAVIER2_TOKEN
        },
        method="POST"
    )
    
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            result = json.loads(response.read())
            return result.get("results", [])
    except Exception as e:
        print(f"Error querying Xavier2: {e}", file=sys.stderr)
        return []

def get_all_memories() -> List[Dict]:
    """Get all memories from Xavier2."""
    import urllib.request
    
    req = urllib.request.Request(
        f"{XAVIER2_URL}/memory/list",
        headers={"X-Xavier2-Token": XAVIER2_TOKEN},
        method="GET"
    )
    
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            result = json.loads(response.read())
            return result.get("memories", [])
    except Exception as e:
        print(f"Error listing memories: {e}", file=sys.stderr)
        return []

def determine_category(path: str, content: str) -> str:
    """Determine the category of a memory document."""
    path_lower = path.lower()
    content_lower = content.lower()
    
    for category, keywords in CATEGORIES.items():
        for keyword in keywords:
            if keyword in path_lower or keyword in content_lower:
                return category
    return "general"

def extract_entities(content: str) -> Dict[str, List[str]]:
    """Extract entities from content for Q&A generation."""
    entities = {
        "persons": [],
        "companies": [],
        "products": [],
        "prices": [],
        "projects": []
    }
    
    # Extract names (capitalized words)
    names = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b', content)
    entities["persons"].extend([n for n in names if len(n.split()) >= 2][:5])
    
    # Extract prices
    prices = re.findall(r'\$?\d{2,4}(?:\s*/\s*(?:mo|month|mes|year|año))?', content)
    entities["prices"] = prices[:5]
    
    # Extract project/product names
    products = re.findall(r'(?:ManteniApp|Cortex|Xavier2|Gestalt|Synapse)', content)
    entities["products"] = list(set(products))[:5]
    
    return entities

def create_qa_pairs(doc: Dict) -> List[Dict]:
    """Create Q&A pairs from a document."""
    pairs = []
    content = doc.get("content", "")
    path = doc.get("path", "")
    category = determine_category(path, content)
    entities = extract_entities(content)
    
    # Create instruction-response pairs based on content
    if category == "business":
        if "SWAL" in content or "SouthWest" in content:
            pairs.append({
                "instruction": "Quién es BELA y qué es SWAL?",
                "response": content,
                "category": category,
                "source": path
            })
    
    if category == "sales" or "pricing" in content.lower():
        if any(p in content for p in ["499", "999", "2499"]):
            pairs.append({
                "instruction": f"Cuáles son los precios de ManteniApp?",
                "response": content,
                "category": category,
                "source": path
            })
    
    if category == "client":
        if "Leonardo" in content or "Rodacenter" in content:
            pairs.append({
                "instruction": "Quién es Leonardo Duque y qué relación tiene con Rodacenter?",
                "response": content,
                "category": category,
                "source": path
            })
    
    if category == "technical":
        if "Xavier2" in content or "Cortex" in content:
            pairs.append({
                "instruction": f"Cuéntame sobre {path.split('/')[0]}",
                "response": content,
                "category": category,
                "source": path
            })
    
    return pairs

def create_reasoning_chain(doc: Dict) -> Optional[Dict]:
    """Create reasoning chain from multi-step context."""
    content = doc.get("content", "")
    path = doc.get("path", "")
    
    # For reasoning-type documents
    if any(kw in path.lower() for kw in ["decision", "analysis", "reasoning"]):
        return {
            "instruction": f"Analiza y explica: {content[:200]}...",
            "response": content,
            "category": "reasoning",
            "source": path
        }
    return None

def dedupe_pairs(pairs: List[Dict]) -> List[Dict]:
    """Remove duplicate or near-duplicate pairs."""
    seen = set()
    unique_pairs = []
    
    for pair in pairs:
        key = pair["instruction"].lower().strip()
        if key not in seen:
            seen.add(key)
            unique_pairs.append(pair)
    
    return unique_pairs

def export_to_jsonl(pairs: List[Dict], output_file: Path):
    """Export pairs to JSONL format."""
    output_file.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_file, "w", encoding="utf-8") as f:
        for pair in pairs:
            f.write(json.dumps(pair, ensure_ascii=False) + "\n")
    
    print(f"Exported {len(pairs)} training pairs to {output_file}")

def main():
    print("=" * 60)
    print("XAVIER2 TRAINING DATA EXPORTER")
    print("=" * 60)
    
    # Get all memories
    print("\n[1/4] Fetching memories from Xavier2...")
    memories = get_all_memories()
    print(f"  Found {len(memories)} memories")
    
    # Also query for specific topics to get more data
    print("\n[2/4] Querying for specific topics...")
    topics = ["SWAL", "ManteniApp", "Leonardo Duque", "Xavier2", "Cortex", "pricing", "sales"]
    for topic in topics:
        results = query_xavier2(topic, limit=10)
        memories.extend(results)
    
    print(f"  Total memories collected: {len(memories)}")
    
    # Create Q&A pairs
    print("\n[3/4] Creating training pairs...")
    all_pairs = []
    
    for doc in memories:
        pairs = create_qa_pairs(doc)
        all_pairs.extend(pairs)
        
        reasoning = create_reasoning_chain(doc)
        if reasoning:
            all_pairs.append(reasoning)
    
    # Deduplicate
    all_pairs = dedupe_pairs(all_pairs)
    print(f"  Created {len(all_pairs)} training pairs")
    
    # Show category distribution
    categories = {}
    for pair in all_pairs:
        cat = pair.get("category", "unknown")
        categories[cat] = categories.get(cat, 0) + 1
    
    print("\n  Category distribution:")
    for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
        print(f"    {cat}: {count}")
    
    # Export
    print("\n[4/4] Exporting to JSONL...")
    export_to_jsonl(all_pairs, OUTPUT_FILE)
    
    print("\n" + "=" * 60)
    print("EXPORT COMPLETE!")
    print("=" * 60)
    print(f"\nNext steps:")
    print(f"1. Review training data at: {OUTPUT_FILE}")
    print(f"2. Upload to Google Colab for fine-tuning")
    print(f"3. Use Phi-3.5 Mini or Llama 3.2 3B with QLoRA")

if __name__ == "__main__":
    main()
