tr"""
Extract comprehensive training data from OpenClaw sessions and memory.
Creates balanced training data for a "synapse" model - one that knows
when to retrieve from memory vs when to generate from reasoning.
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional

# Configuration
MEMORY_DIR = Path("memory")
CORE_FILES = ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md", "TOOLS.md", "IDENTITY.md"]
TRANSCRIPT_PATH = Path(r"C:\Users\belal\.openclaw\agents\ventas\sessions\4d8e1f61-7991-4737-8b61-6a4ced3ee158.jsonl")
OUTPUT_FILE = Path("E:/datasetsDrive/training/swal_training_comprehensive.jsonl")

# Training categories for synapse balance
CATEGORIES = {
    # Memory retrieval patterns - model learns WHEN to retrieve
    "memory_trigger": ["consultar", "revisar", "buscar en memoria", "qué tengo registrado", "qué sabes de"],

    # Generation patterns - model learns WHEN to generate
    "generation": ["explica cómo", "analiza", "por qué", "cuéntame sobre", "cómo harías"],

    # Synthesis patterns - model learns HOW to combine
    "synthesis": ["basándote en", "según mi memoria", "结合", "integrando"],

    # Decision patterns
    "decision": ["debería", "qué recomiendas", "mejor opción", "estrategia"],

    # Core knowledge (exact facts from memory)
    "knowledge_exact": ["quién es", "qué es", "cuál es el precio", "dónde está"],

    # Reasoning patterns
    "reasoning": ["piensa paso a paso", "analiza el problema", "razonamiento"],

    # Operation patterns (how to do things)
    "operations": ["cómo hago", "cómo configuro", "cómo ejecuto", "comando"],

    # Client/sales patterns
    "sales": ["RFI", "propuesta", "cotización", "cliente", "Leonardo", "Rodacenter"],

    # Technical patterns
    "technical": ["código", "script", "API", "modelo", "OpenClaw", "Cortex", "Xavier"],
}

def load_memory_files() -> List[Dict]:
    """Load all memory markdown files."""
    memories = []

    # Load daily memories
    if MEMORY_DIR.exists():
        for md_file in MEMORY_DIR.glob("*.md"):
            if md_file.name == ".dreams":
                continue
            try:
                content = md_file.read_text(encoding='utf-8')
                memories.append({
                    "source": f"memory/{md_file.name}",
                    "content": content,
                    "type": "daily_memory"
                })
            except Exception as e:
                print(f"Error reading {md_file}: {e}")

    # Load core files
    for core_file in CORE_FILES:
        if Path(core_file).exists():
            try:
                content = Path(core_file).read_text(encoding='utf-8')
                memories.append({
                    "source": core_file,
                    "content": content,
                    "type": "core_file"
                })
            except Exception as e:
                print(f"Error reading {core_file}: {e}")

    return memories

def load_transcript() -> List[Dict]:
    """Load conversation transcript and extract Q&A pairs."""
    pairs = []

    if not TRANSCRIPT_PATH.exists():
        print(f"Transcript not found: {TRANSCRIPT_PATH}")
        return pairs

    try:
        with open(TRANSCRIPT_PATH, 'r', encoding='utf-8') as f:
            for line in f:
                try:
                    entry = json.loads(line.strip())
                    pairs.append(entry)
                except:
                    continue
    except Exception as e:
        print(f"Error loading transcript: {e}")

    return pairs

def extract_qa_from_transcript(transcript: List[Dict]) -> List[Dict]:
    """Extract Q&A pairs from conversation transcript."""
    qa_pairs = []

    # Group by user-assistant turns
    user_msgs = []
    assistant_msgs = []

    for entry in transcript:
        role = entry.get('role', '')
        content = entry.get('content', '')

        # Handle content as list or string
        if isinstance(content, list):
            text_content = ' '.join([c.get('text', '') if isinstance(c, dict) else str(c) for c in content])
        else:
            text_content = str(content)

        if role == 'user' and text_content.strip():
            user_msgs.append(text_content)
        elif role == 'assistant' and text_content.strip():
            assistant_msgs.append(text_content)

    # Create pairs from consecutive turns
    min_len = min(len(user_msgs), len(assistant_msgs))
    for i in range(min_len):
        # Skip very short messages (likely system messages)
        if len(user_msgs[i]) > 20 and len(assistant_msgs[i]) > 20:
            qa_pairs.append({
                "instruction": user_msgs[i].strip(),
                "response": assistant_msgs[i].strip(),
                "source": "transcript",
                "index": i
            })

    return qa_pairs

def extract_memory_knowledge(memories: List[Dict]) -> List[Dict]:
    """Extract structured knowledge from memory files."""
    knowledge_pairs = []

    for mem in memories:
        content = mem.get('content', '')
        source = mem.get('source', 'unknown')

        # Extract specific facts based on patterns
        lines = content.split('\n')

        for line in lines:
            line = line.strip()

            # Skip headers and short lines
            if len(line) < 30 or line.startswith('#'):
                continue

            # Look for structured knowledge
            # Pattern: "Key: Value" or "Key - Value"
            if ': ' in line or ' - ' in line:
                parts = line.split(': ') if ': ' in line else line.split(' - ')
                if len(parts) >= 2:
                    key = parts[0].strip()
                    value = ': '.join(parts[1:]).strip()

                    if len(key) > 3 and len(value) > 20:
                        # Create Q&A from structured data
                        if any(kw in key.lower() for kw in ['quien', 'quién', 'name', 'nombre', 'quienes']):
                            knowledge_pairs.append({
                                "instruction": f"Quién es {key}?",
                                "response": value,
                                "category": "person_knowledge",
                                "source": source
                            })
                        elif any(kw in key.lower() for kw in ['precio', 'price', 'costo', 'cost']):
                            knowledge_pairs.append({
                                "instruction": f"Cuál es el precio de {key}?",
                                "response": value,
                                "category": "pricing_knowledge",
                                "source": source
                            })
                        elif any(kw in key.lower() for kw in ['project', 'proyecto', 'producto', 'modelo']):
                            knowledge_pairs.append({
                                "instruction": f"Qué es {key}?",
                                "response": value,
                                "category": "entity_knowledge",
                                "source": source
                            })

            # Look for Q&A in markdown format
            if line.startswith('**') and line.endswith('**'):
                # Bold text might be a question
                question = line.strip('* ')
                if '?' in question:
                    # Find answer in next lines
                    idx = lines.index(line)
                    if idx + 1 < len(lines):
                        answer = lines[idx + 1].strip()
                        if answer and not answer.startswith('#'):
                            knowledge_pairs.append({
                                "instruction": question,
                                "response": answer,
                                "category": "qa_format",
                                "source": source
                            })

    return knowledge_pairs

def categorize_by_synapse_role(qa_pairs: List[Dict]) -> Dict[str, List[Dict]]:
    """Categorize pairs by their role in the memory-generation synapse."""
    categorized = {
        # Pairs where model should trigger memory retrieval
        "memory_retrieval": [],

        # Pairs where model should generate from knowledge
        "knowledge_generation": [],

        # Pairs where model should synthesize (memory + generation)
        "synthesis": [],

        # Pairs for core SWAL knowledge
        "swal_core": [],

        # Pairs for operations/how-to
        "operations": [],

        # Pairs for reasoning chains
        "reasoning": [],
    }

    for pair in qa_pairs:
        instruction = pair.get('instruction', '').lower()
        response = pair.get('response', '').lower()

        # Categorize based on instruction patterns
        if any(kw in instruction for kw in ['revisar', 'consultar', 'buscar', 'qué tienes', 'mi memoria']):
            categorized["memory_retrieval"].append(pair)
        elif any(kw in instruction for kw in ['explica', 'analiza', 'cómo', 'por qué']):
            if any(kw in response for kw in ['memoria', 'registrado', 'según']):
                categorized["synthesis"].append(pair)
            else:
                categorized["knowledge_generation"].append(pair)
        elif any(kw in instruction for kw in ['swal', 'bel a', 'manteniapp', 'cortex', 'xavier']):
            categorized["swal_core"].append(pair)
        elif any(kw in instruction for kw in ['cómo hago', 'cómo configuro', 'comando', 'ejecuta']):
            categorized["operations"].append(pair)
        elif any(kw in instruction for kw in ['piensa', 'analiza', 'razonamiento', 'paso']):
            categorized["reasoning"].append(pair)
        else:
            # Default to knowledge generation
            categorized["knowledge_generation"].append(pair)

    return categorized

def add_synthesis_training_pairs() -> List[Dict]:
    """Add training pairs specifically for the synapse concept."""
    synthesis_pairs = [
        # Memory retrieval triggers
        {
            "instruction": "Necesito información sobre Rodacenter, qué tienes registrado?",
            "response": "Déjame consultar mi memoria... Según tengo registrado, Rodacenter es un cliente potencial ubicado en Antofagasta, Chile. Son un socio del laboratorio y están interesados en ManteniApp para monitoreo de maquinaria. Su empresa relacionada es tripro.cl. Leonardo Duque es el contacto principal como vendedor/socio externo de SWAL.",
            "category": "synapse_memory_trigger",
            "source": "synthetic"
        },
        {
            "instruction": "Qué sabes sobre los proyectos actuales de SWAL?",
            "response": "Tengo registro de varios proyectos activos: gestalt-rust (Rust-based system), manteniapp (monitoreo maquinaria), synapse-agentic (agente autonomous), domus-otec (proyecto laboratorio), worldexams, edge-hive, y tripro_landing_page_astro. Los proyectos de desarrollo están en E:\\scripts-python\\ para mantener organización adecuada.",
            "category": "synapse_memory_trigger",
            "source": "synthetic"
        },
        {
            "instruction": "Cuéntame sobre los precios de ManteniApp",
            "response": "Basándome en mi memoria, ManteniApp tiene tres planes: Starter a $499/mes (básico), Pro a $999/mes (profesional), y Enterprise a $2499/mes (enterprise). El producto está enfocado en monitoreo predictivo de maquinaria industrial usando AI.",
            "category": "synapse_synthesis",
            "source": "synthetic"
        },
        {
            "instruction": "Qué configuración de APIs tiene SWAL?",
            "response": "SWAL tiene configurados tres providers de búsqueda web: Tavily (1000/mes, bueno para AI answers), Brave Search (2000/mes, rápido), y MiniMax via gateway. Cuando las APIs llegan al 80% hay warning y al 90% hace fallback automático a CLI (Claude Code > Qwen > Gemini).",
            "category": "synapse_synthesis",
            "source": "synthetic"
        },
        # Generation without retrieval
        {
            "instruction": "Cómo harías para generar un RFI para un cliente nuevo?",
            "response": "Para generar un RFI, seguiría estos pasos: 1) Identificar al cliente y sus necesidades específicas, 2) Revisar si tenemos info previa en memoria sobre él, 3) Usar el template en skills/sales-pro/rfi-template.md, 4) Personalizar según el producto de interés (ManteniApp, Software Factory, Cortex), 5) Guardar el RFI en prospects/README.md para seguimiento.",
            "category": "generation_no_retrieval",
            "source": "synthetic"
        },
        {
            "instruction": "Por qué SWAL usa Cortex como sistema de memoria central?",
            "response": "SWAL usa Cortex (versión 0.4.1 con backend vectorial en localhost:8006) porque proporciona: 1) Memoria centralizada accesible por todos los agentes, 2) Búsqueda semántica rápida via embeddings, 3) Categorización automática (technical, client, operations, sales), 4) Sincronización con archivos locales, 5) Mucho más rápido que mantener todo en archivos planos. Es el 'cerebro compartido' del sistema.",
            "category": "knowledge_generation",
            "source": "synthetic"
        },
        # Decision patterns
        {
            "instruction": "Qué modelo de lenguaje debería usar para diferentes tareas?",
            "response": "Depende de la tarea: 1) Para investigación profunda → MiniMax con Tavily, 2) Para coding → Codex (con --dangerously-bypass-approvals-and-sandbox), 3) Para fallback rápido → Qwen Coder o Claude Code, 4) Para generación de imágenes → DALL-E/Midjourney, 5) Para audio → Whisper local o Groq API. La estrategia es usar el mejor disponible según el límite de uso.",
            "category": "decision_pattern",
            "source": "synthetic"
        },
        # Operations
        {
            "instruction": "Cómo configuro un nuevo cron job en OpenClaw?",
            "response": "Para crear un cron job en OpenClaw: 1) Define el schedule (cron expression o interval), 2) Define el payload (systemEvent para main session o agentTurn para isolated), 3) Usa sessionTarget según necesidad (main/isolated/current/session:<id>), 4) Añade delivery para notificar al completar, 5) Considera failureAlert para monitorizar errores. Los jobs activos se listan con openclaw cron list.",
            "category": "operations",
            "source": "synthetic"
        },
        {
            "instruction": "Cuál es la estructura correcta de proyectos SWAL?",
            "response": "La regla es: proyectos de desarrollo VAN en E:\\scripts-python\\ (nunca en C:\\Users\\belal\\clawd\\). Estructura correcta: E:\\scripts-python\\gestalt-rust\\, E:\\scripts-python\\manteniapp\\, E:\\scripts-python\\synapse-agentic\\, etc. El directorio C:\\Users\\belal\\clawd\\ es SOLO para configuración de OpenClaw, no para código de proyectos.",
            "category": "swal_core",
            "source": "synthetic"
        },
        # Reasoning chains
        {
            "instruction": "Analiza: Tenemos un cliente nuevo (Rodacenter) interesado en ManteniApp. Qué pasos sigues?",
            "response": "Pensándolo paso a paso: 1) CONSULTAR MEMORIA: Buscar si existe info previa sobre Rodacenter (ya tenemos: Antofagasta, Chile, socio laboratorio, interesados en ManteniApp). 2) EVALUAR NECESIDADES: El cliente necesita monitoreo de maquinaria - confirmar qué tipo de equipos y escala. 3) PRESENTAR PRODUCTO: Explicar los tres planes (Starter/Pro/Enterprise) según su necesidad. 4) GENERAR RFI: Usar sales-pro para crear documento formal. 5) HACER FOLLOW-UP: Registrar en prospects para tracking. 6) SI ES IMPORTANTE: Guardar decisión en Cortex.",
            "category": "reasoning_chain",
            "source": "synthetic"
        },
    ]
    return synthesis_pairs

def deduplicate_pairs(pairs: List[Dict]) -> List[Dict]:
    """Remove duplicate or near-duplicate pairs."""
    seen = set()
    unique_pairs = []

    for pair in pairs:
        # Normalize for comparison
        key = pair.get('instruction', '').lower().strip()
        # Remove punctuation and extra spaces
        key = ''.join(c for c in key if c.isalnum() or c.isspace())
        key = ' '.join(key.split())

        if key not in seen and len(key) > 10:
            seen.add(key)
            unique_pairs.append(pair)

    return unique_pairs

def main():
    print("=" * 60)
    print("SWAL TRAINING DATA EXTRACTOR")
    print("Building comprehensive dataset for synapse model")
    print("=" * 60)

    all_pairs = []

    # 1. Load memory files
    print("\n[1/5] Loading memory files...")
    memories = load_memory_files()
    print(f"  Found {len(memories)} memory sources")

    # 2. Extract knowledge from memory
    print("\n[2/5] Extracting knowledge from memories...")
    knowledge_pairs = extract_memory_knowledge(memories)
    print(f"  Extracted {len(knowledge_pairs)} knowledge pairs")

    # 3. Load and process transcript
    print("\n[3/5] Loading conversation transcript...")
    transcript = load_transcript()
    print(f"  Loaded {len(transcript)} transcript entries")

    qa_pairs = extract_qa_from_transcript(transcript)
    print(f"  Extracted {len(qa_pairs)} Q&A pairs from conversation")

    # 4. Add synthesis training pairs
    print("\n[4/5] Adding synapse training pairs...")
    synthesis_pairs = add_synthesis_training_pairs()
    print(f"  Added {len(synthesis_pairs)} synthesis pairs")

    # 5. Combine all pairs
    all_pairs.extend(knowledge_pairs)
    all_pairs.extend(qa_pairs)
    all_pairs.extend(synthesis_pairs)

    # Deduplicate
    all_pairs = deduplicate_pairs(all_pairs)

    print(f"\n  Total pairs before dedup: {len(knowledge_pairs) + len(qa_pairs) + len(synthesis_pairs)}")
    print(f"  Total pairs after dedup: {len(all_pairs)}")

    # Categorize
    categorized = categorize_by_synapse_role(all_pairs)

    print("\n  Category distribution:")
    for cat, pairs in categorized.items():
        if pairs:
            print(f"    {cat}: {len(pairs)}")

    # Save
    print("\n[5/5] Saving to JSONL...")
    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)

    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        for pair in all_pairs:
            f.write(json.dumps(pair, ensure_ascii=False) + '\n')

    print(f"\n  Saved {len(all_pairs)} training pairs to:")
    print(f"  {OUTPUT_FILE}")

    print("\n" + "=" * 60)
    print("EXTRACTION COMPLETE!")
    print("=" * 60)

    print("\nNext steps:")
    print("1. Review the dataset for quality")
    print("2. Add more pairs if needed")
    print("3. Upload to Colab for training")
    print("4. Monitor for sensitive info exposure")

if __name__ == "__main__":
    main()