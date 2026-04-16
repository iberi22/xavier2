#!/usr/bin/env python3
"""
Build comprehensive SWAL Synapse training dataset.
500+ examples covering all operation categories.
"""

import json
import random
from pathlib import Path

OUTPUT_FILE = Path("E:/datasetsDrive/training/synapse_training.jsonl")

# Core SWAL knowledge base
SWAL_KNOWLEDGE = {
    "bela": {
        "name": "BELA",
        "role": "Fundador y líder de Southwest AI Labs (SWAL)",
        "timezone": "America/Bogota",
        "products": ["ManteniApp", "Cortex", "Software Factory"],
        "projects": ["gestalt-rust", "manteniapp", "synapse-agentic", "domus-otec"],
        "contact": "Telegram: 2076598024"
    },
    "leonardo_duque": {
        "role": "Vendedor/socio externo SWAL",
        "client": "Rodacenter",
        "location": "Antofagasta, Chile",
        "access": "Bot Docker separado",
        "workflow": "Sincronizar TAREA, no conversaciones"
    },
    "rodacenter": {
        "location": "Antofagasta, Chile",
        "type": "Cliente potencial",
        "relation": "Socio del laboratorio",
        "interest": "ManteniApp - monitoreo de maquinaria",
        "company_url": "tripro.cl"
    },
    "manteniapp": {
        "type": "Plataforma de monitoreo de maquinaria con AI",
        "pricing": {"starter": "$499/mes", "pro": "$999/mes", "enterprise": "$2499/mes"},
        "focus": "Mantenimiento predictivo",
        "demo_url": "tripro.cl/manteniapp",
        "repo": "iberi22/tripro_landing_page_astro"
    },
    "cortex": {
        "type": "Sistema de memoria IA centralizado",
        "version": "0.4.1",
        "url": "localhost:8006",
        "categories": ["technical", "client", "operations", "sales"],
        "backend": "vectorial"
    },
    "xavier2": {
        "type": "Agente principal SWAL",
        "host": "EditorOne (Windows)",
        "role": "Ventas y operaciones",
        "skills": ["sales-pro", "src-generator"]
    },
    "openclaw": {
        "type": "Agent framework",
        "config_dir": "C:\\Users\\belal\\clawd\\",
        "workspace": "C:\\Users\\belal\\clawd\\agents\\ventas"
    },
    "projects_location": {
        "rule": "Proyectos en E:\\scripts-python\\ (nunca en C:\\Users\\belal\\clawd\\)",
        "valid": ["gestalt-rust", "manteniapp", "synapse-agentic", "domus-otec"]
    }
}

def generate_verification_pairs(n=150):
    """Generate pre-action verification pairs."""
    pairs = []
    
    templates = [
        {
            "instruction": "Verifica esta acción: enviar email a {contact} sobre {topic}",
            "context_templates": [
                "Último contacto hace {days} días, {status}",
                "Cliente nuevo, primera interacción",
                "Follow-up después de propuesta",
                "Sin contacto previo reciente"
            ],
            "outcomes": [
                "VERIFIED - Procede. {reason}",
                "MODIFIED - {modification}. {reason}",
                "BLOCKED - {block_reason}"
            ]
        },
        {
            "instruction": "Verifica antes de ejecutar: {action}",
            "context_templates": [
                "Usuario: {user}",
                "Contexto: {context}",
                "Historial: {history}"
            ],
            "outcomes": [
                "VERIFIED - Acción segura y apropiada.",
                "MODIFIED - {modification}. Recomendación: {rec}",
                "BLOCKED - {block_reason}. Requiere confirmación."
            ]
        },
        {
            "instruction": "Confirma si esta acción es correcta: {action}",
            "context_templates": [
                "Se requiere {requirement}",
                "Impacto: {impact}",
                "Prioridad: {priority}"
            ],
            "outcomes": [
                "VERIFIED - Correcto proceder.",
                "VERIFIED with caution - {caution}",
                "BLOCKED - {block_reason}"
            ]
        }
    ]
    
    contacts = ["Leonardo Duque", "cliente Rodacenter", "equipo SWAL", "nuevo prospecto"]
    topics = ["ManteniApp pricing", "seguimiento demo", "nueva propuesta", "actualización proyecto"]
    statuses = ["sin respuesta", "interesado", "en evaluación", "listo para comprar"]
    days_range = [1, 3, 7, 14, 30]
    
    actions = [
        "Crear RFI para nuevo cliente",
        "Enviar cotización de ManteniApp",
        "Actualizar estado de prospecto en Cortex",
        "Ejecutar script de backup",
        "Crear cron job para monitoreo",
        "Enviar follow-up a Leonardo",
        "Generar propuesta comercial",
        "Crear nuevo skill en OpenClaw",
        "Ejecutar análisis de security"
    ]
    
    for i in range(n):
        template = random.choice(templates)
        
        instruction = template["instruction"].format(
            contact=random.choice(contacts),
            topic=random.choice(topics),
            action=random.choice(actions)
        )
        
        context = random.choice(template["context_templates"]).format(
            days=random.choice(days_range),
            status=random.choice(statuses),
            user="BELA",
            context="consulta de ventas",
            history="interacción previa positiva"
        )
        
        outcome = random.choice(template["outcomes"])
        
        if outcome.startswith("VERIFIED"):
            pairs.append({
                "instruction": f"{instruction}\nContexto: {context}",
                "response": outcome,
                "category": "verification",
                "trigger": "action_verification"
            })
        elif outcome.startswith("MODIFIED"):
            pairs.append({
                "instruction": f"{instruction}\nContexto: {context}",
                "response": outcome,
                "category": "verification",
                "trigger": "action_modification"
            })
        else:
            pairs.append({
                "instruction": f"{instruction}\nContexto: {context}",
                "response": outcome,
                "category": "verification",
                "trigger": "action_blocked"
            })
    
    return pairs

def generate_orchestration_pairs(n=100):
    """Generate multi-step operation orchestration pairs."""
    pairs = []
    
    tasks = [
        ("Setup nuevo cliente en el sistema", [
            "1. Consultar memoria para info previa del cliente",
            "2. Crear registro en Cortex con categoría client",
            "3. Generar RFI inicial usando sales-pro",
            "4. Configurar follow-up en cron job"
        ]),
        ("Ejecutar auditoría de seguridad diaria", [
            "1. Verificar estado de repositorios iberi22",
            "2. Revisar logs de GitHub Actions",
            "3. Buscar secrets expuestos en commits recientes",
            "4. Si se detecta: alerta Telegram + crear issue security/critical"
        ]),
        ("Generar propuesta comercial para ManteniApp", [
            "1. Consultar precios actuales en MEMORY",
            "2. Obtener requisitos del cliente desde Cortex",
            "3. Usar proposal-template.md de sales-pro",
            "4. Personalizar según tier seleccionado",
            "5. Guardar en prospects/README.md"
        ]),
        ("Onboarding nuevo proyecto de software", [
            "1. Crear estructura en E:\\scripts-python\\{project}",
            "2. Inicializar repo Git",
            "3. Registrar en MEMORY.md",
            "4. Configurar cron de sync si aplica"
        ]),
        (" Investigación de prospecto", [
            "1. Buscar info existente en Cortex",
            "2. Web research sobre empresa",
            "3. Guardar hallazgos en Cortex (category: client)",
            "4. Evaluar fit con productos SWAL"
        ])
    ]
    
    for i in range(n):
        task, steps = random.choice(tasks)
        
        orchestration = "Pasos:\n" + "\n".join(f"{j+1}. {s}" for j, s in enumerate(steps))
        orchestration += "\n\nCheckpoint después de cada paso."
        
        evaluation = random.choice([
            "Plan ORCHESTRATED correctamente.Procede con ejecución.",
            "Plan optimizado: pasos 2 y 3 pueden ejecutarse en paralelo.",
            "Añadir verificación antes del paso final."
        ])
        
        pairs.append({
            "instruction": f"Orquesta esta tarea: {task}",
            "response": f"{orchestration}\n\nEvaluación: {evaluation}",
            "category": "orchestration",
            "trigger": "multi_step_task"
        })
    
    return pairs

def generate_synthesis_pairs(n=125):
    """Generate memory + generation synthesis pairs."""
    pairs = []
    
    queries = [
        "Qué sabes de {entity}?",
        "Dame información sobre {entity} incluyendo lo más reciente",
        "Qué tenemos registrado de {entity}?",
        "Cuál es el estado actual de {entity}?"
    ]
    
    entities = list(SWAL_KNOWLEDGE.keys())
    
    for i in range(n):
        entity_key = random.choice(entities)
        entity_data = SWAL_KNOWLEDGE[entity_key]
        
        query = random.choice(queries).format(entity=entity_key.replace("_", " ").title())
        
        if entity_key == "bela":
            response = f"Según mi memoria, BELA es {entity_data['role']}. Opera desde Colombia ({entity_data['timezone']}) y gestiona múltiples proyectos de software. Productos: {', '.join(entity_data['products'])}. proyectos activos: {', '.join(entity_data['projects'])}."
        elif entity_key == "leonardo_duque":
            response = f"Leonardo Duque es {entity_data['role']}, trabaja principalmente con {entity_data['client']} ({entity_data['location']}). Acceso: {entity_data['access']}. Workflow: sincronizar {entity_data['workflow']}."
        elif entity_key == "manteniapp":
            tiers = ", ".join([f"{k}: {v}" for k, v in entity_data['pricing'].items()])
            response = f"ManteniApp es {entity_data['type']}. Enfoque: {entity_data['focus']}. Precios: {tiers}. Demo: {entity_data['demo_url']}"
        elif entity_key == "cortex":
            response = f"Cortex es {entity_data['type']}. Versión {entity_data['version']} con backend {entity_data['backend']} en {entity_data['url']}. Categorías: {', '.join(entity_data['categories'])}."
        elif entity_key == "rodacenter":
            response = f"Rodacenter es {entity_data['type']} en {entity_data['location']}. {entity_data['relation']}. Interesado en: {entity_data['interest']}. Empresa: {entity_data['company_url']}"
        else:
            response = f"{entity_key}: {str(entity_data)}"
        
        synthesis_marker = random.choice([
            "Basándome en lo que tengo registrado...",
            "Según mi memoria...",
            "Tengo información de que...",
            "Consultando Cortex, encuentro que..."
        ])
        
        pairs.append({
            "instruction": query,
            "response": f"{synthesis_marker} {response}",
            "category": "synthesis",
            "trigger": "memory_plus_generation"
        })
    
    return pairs

def generate_generation_pairs(n=75):
    """Pure generation when memory doesn't have info."""
    pairs = []
    
    instructions = [
        "Cómo harías para {task}?",
        "Explica el proceso para {task}",
        "Qué pasos sigues para {task}?",
        "Describe cómo ejecutar {task} de forma óptima"
    ]
    
    tasks = [
        "crear un RFI desde cero",
        "generar propuesta comercial",
        "hacer research de un cliente potencial",
        "configurar un nuevo cron job",
        "ejecutar backup de memoria",
        "analizar conversión de prospectos",
        "generar reporte semanal de decisiones",
        "auditar seguridad de repos",
        "sincronizar tareas con Leonardo"
    ]
    
    responses = [
        "Para {task}, seguiría: 1) Identificar objetivo, 2) Consultar templates existentes, 3) Personalizar según contexto, 4) Ejecutar y verificar resultado, 5) Documentar en memoria.",
        "El proceso sería: Primero reviso si hay info previa en Cortex, luego busco template correspondiente en skills/, ejecuto con adaptaciones, finalmente guardo resultado.",
        "Pasos: 1) Definir alcance, 2) Revisar recursos disponibles (skills, memory), 3) Ejecutar con verificación intermedio, 4) Guardar resultado y actualizar memoria.",
        "Para {task} recomendo: revisar MEMORY.md para contexto, usar skill apropiado si existe, ejecutar y validar output, guardar en Cortex si es información valiosa."
    ]
    
    for i in range(n):
        task = random.choice(tasks)
        instruction = random.choice(instructions).format(task=task)
        response_template = random.choice(responses)
        response = response_template.format(task=task)
        
        pairs.append({
            "instruction": instruction,
            "response": response,
            "category": "generation",
            "trigger": "pure_generation"
        })
    
    return pairs

def generate_retrieval_trigger_pairs(n=50):
    """Pairs teaching when to consult Cortex."""
    pairs = []
    
    instructions = [
        "Necesito info sobre {entity}",
        "Qué tienes de {entity}?",
        "Consultar {entity}",
        "Revisa lo que tenemos de {entity}"
    ]
    
    entities = ["Leonardo Duque", "Rodacenter", "ManteniApp", "gestalt-rust", "ventas ManteniApp"]
    
    for i in range(n):
        entity = random.choice(entities)
        instruction = random.choice(instructions).format(entity=entity)
        
        trigger = random.choice([
            "TRIGGER_RETRIEVAL - Buscar en Cortex categoría: ",
            "RETRIEVAL_SIGNAL - Consultar memoria: ",
            "CONSULT_MEMORY - Revisar registros de "
        ])
        
        category = random.choice(["client", "product", "projects", "sales"])
        
        pairs.append({
            "instruction": instruction,
            "response": f"{trigger}{category}",
            "category": "retrieval_trigger",
            "trigger": "when_to_consult_memory"
        })
    
    return pairs

def generate_decision_pairs(n=50):
    """Decision-making pairs."""
    pairs = []
    
    scenarios = [
        ("Cliente pregunta precio de ManteniApp", "Starter $499, Pro $999, Enterprise $2499 - explicar diferencias y recomendar según necesidad"),
        ("Leonardo pide información de demo", "Enviar link tripro.cl/manteniapp, confirmar fecha, preparar materiales"),
        (" prospecto nuevo pregunta por timeline", "Depende del tier: Starter inmediata, Pro 1 semana, Enterprise 2-4 semanas"),
        ("Se detecta secret expuesto en repo", "Seguir protocolo: 1) Alertar Telegram, 2) Crear issue security/critical, 3) Auto-fix git rm --cached, 4) Crear PR"),
        ("Cliente pregunta por integración API", "Software Factory puede desarrollar integración custom - agendar llamada para spec")
    ]
    
    for i in range(n):
        scenario, decision = random.choice(scenarios)
        
        pairs.append({
            "instruction": f"Decisión: {scenario}",
            "response": f"RECOMENDACIÓN: {decision}",
            "category": "decision",
            "trigger": "operational_decision"
        })
    
    return pairs

def generate_operations_pairs(n=50):
    """How-to operational pairs."""
    pairs = []
    
    how_tos = [
        ("cómo hago un git commit con mensaje descriptivo", "git add . && git commit -m 'feat: descripción corta de cambios'"),
        ("cómo creo un cron job en OpenClaw", "Usar openclaw cron add con schedule y payload apropiados. Definir sessionTarget según necesidad (main/isolated)."),
        ("cómo verifico estado de Ollama", "ollama list para ver modelos, ollama run {model} para probar."),
        ("cómo exporto data de Cortex", "POST a /memory/export con query y formato. Recibirás JSON con resultados."),
        ("cómo ejecuto el Project Synthesizer", "El cron corre cada 6h automáticamente, o ejecutar manualmente via sessions_send a Codex."),
        ("cómo genero un SRC document", "Usar skill src-generator: skills/src-generator/generate-src.py con specs del proyecto."),
        ("cómo hago backup de memoria", "Sincronizar a Git (git add memory/ && git commit), también guardar en E:\\datasetsDrive\\backup\\"),
        ("cómo limpio sessions antiguas", "openclaw sessions list, luego eliminar con cuidado. Mantener solo activas.")
    ]
    
    for i in range(n):
        how, answer = random.choice(how_tos)
        
        pairs.append({
            "instruction": f"{how}?",
            "response": answer,
            "category": "operations",
            "trigger": "how_to"
        })
    
    return pairs

def generate_reasoning_pairs(n=50):
    """Reasoning chain pairs."""
    pairs = []
    
    scenarios = [
        "Un cliente dice que el precio es muy alto. Cómo respondes?",
        "Necesito decidir entre enviar email o esperar. Qué consideras?",
        "Analiza: tenemos 3 prospectos en pipeline, cuál priorizo?",
        "Por qué es importante mantener memoria actualizada?"
    ]
    
    for i in range(n):
        scenario = random.choice(scenarios)
        
        reasoning = random.choice([
            "Pensándolo paso a paso: 1) Identificar variables clave, 2) Evaluar restricciones, 3) Considerar historial, 4) Proyectar outcomes, 5) Decidir.",
            "Análisis: Primero necesito contexto de memoria, luego evaluar opciones, finalmente producir recomendación con explicación.",
            "Razonamiento: a) ¿Qué información tengo? b) ¿Qué falta? c) ¿Cuál es el objetivo? d) ¿Qué impedimentos hay? e) Conclusión."
        ])
        
        pairs.append({
            "instruction": scenario,
            "response": f"{reasoning} Considerando el contexto actual de SWAL y los principios definidos.",
            "category": "reasoning",
            "trigger": "step_by_step"
        })
    
    return pairs

def generate_swal_core_pairs(n=75):
    """Core SWAL knowledge pairs."""
    pairs = []
    
    knowledge = [
        ("Quién es BELA?", "BELA es el fundador de Southwest AI Labs (SWAL), empresa de software con productos ManteniApp, Cortex y Software Factory. Opera desde Colombia."),
        ("Qué es ManteniApp?", "ManteniApp es la plataforma de monitoreo de maquinaria con AI de SWAL. Tiene 3 planes: Starter $499, Pro $999, Enterprise $2499."),
        ("Quién es Leonardo Duque?", "Leonardo Duque es socio/vendedor externo de SWAL que trabaja con Rodacenter en Chile. Sincroniza tareas, no conversaciones."),
        ("Dónde está Rodacenter?", "Rodacenter está en Antofagasta, Chile. Son un cliente potencial interesado en ManteniApp."),
        ("Qué proyectos tiene SWAL?", "Gestalt-rust, Manteniapp, Synapse-agentic, Domus-otec, Worldexams, Edge-hive. Ubicación: E:\\scripts-python\\."),
        ("Qué es Cortex?", "Cortex es el sistema de memoria centralizado para agentes SWAL. Versión 0.4.1 con backend vectorial en localhost:8006."),
        ("Cuál es la regla de proyectos?", "Los proyectos de desarrollo VAN en E:\\scripts-python\\, nunca en C:\\Users\\belal\\clawd\\ (ese es solo para OpenClaw)."),
        ("Qué API keys tiene SWAL?", "Tavily (1000/mes), Brave Search (2000/mes), MiniMax via gateway. Fallbacks: Claude Code > Qwen > Gemini."),
        ("Qué cron jobs hay activos?", "Project Synthesizer (6h), Weekly Report (Lunes 8AM), Security Audit (8AM), GitHub Actions Analyzer (4h), GitHub Monitor (1h)."),
        ("Cuántos productos tiene SWAL?", "Tres principales: ManteniApp (monitoreo AI), Software Factory (desarrollo a medida), Cortex (memoria IA).")
    ]
    
    for i in range(n):
        q, a = random.choice(knowledge)
        pairs.append({
            "instruction": q,
            "response": a,
            "category": "swal_core",
            "trigger": "core_knowledge"
        })
    
    return pairs

def main():
    print("=" * 60)
    print("BUILDING SWAL SYNAPSE TRAINING DATASET")
    print("Target: 500+ examples")
    print("=" * 60)
    
    all_pairs = []
    
    # Generate all categories
    categories = [
        ("verification", generate_verification_pairs, 150),
        ("orchestration", generate_orchestration_pairs, 100),
        ("synthesis", generate_synthesis_pairs, 125),
        ("generation", generate_generation_pairs, 75),
        ("retrieval_trigger", generate_retrieval_trigger_pairs, 50),
        ("decision", generate_decision_pairs, 50),
        ("operations", generate_operations_pairs, 50),
        ("reasoning", generate_reasoning_pairs, 50),
        ("swal_core", generate_swal_core_pairs, 75),
    ]
    
    total = 0
    for name, generator, count in categories:
        pairs = generator(count)
        all_pairs.extend(pairs)
        total += len(pairs)
        print(f"  {name}: {len(pairs)} pairs")
    
    # Shuffle
    random.shuffle(all_pairs)
    
    # Save
    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        for pair in all_pairs:
            f.write(json.dumps(pair, ensure_ascii=False) + '\n')
    
    print(f"\nTotal: {total} pairs")
    print(f"Saved to: {OUTPUT_FILE}")
    
    # Stats
    categories_count = {}
    for pair in all_pairs:
        cat = pair.get("category", "unknown")
        categories_count[cat] = categories_count.get(cat, 0) + 1
    
    print("\nCategory distribution:")
    for cat, count in sorted(categories_count.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")

if __name__ == "__main__":
    main()