# Project Status Agent

Agent que obtiene el estado de los proyectos desde GitHub API y lo guarda en Xavier.

## Uso

### Ejecución única
```bash
cd E:\scripts-python\xavier
python project_status_agent.py
```

### Modo polling (actualiza cada 5 minutos)
```bash
python project_status_agent.py --poll
```

### Intervalo personalizado
```bash
python project_status_agent.py --poll --interval 600  # cada 10 minutos
```

## Datos que obtiene

- **CI Status** - Estado de GitHub Actions (passing/failing/unknown)
- **Issues Count** - Cantidad de issues abiertos
- **Last Commit** - Fecha del último commit
- **Blockers** - Traducción desde STATUS.md

## Paths en Xavier

- Overview: `sweat-operations/projects/overview`
- Proyecto individual: `sweat-operations/projects/{name}/status`

## Requisitos

- Python 3.8+
- requests library
- Acceso a Xavier en localhost:8003
- GitHub API token (opcional, para mayor rate limit)
