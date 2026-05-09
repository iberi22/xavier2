# PLANNING.md

Gestión de Planificación: Xavier
Última actualización: 2026-04-14

## Visión y Alcance

Xavier es un motor de memoria cognitiva para agentes IA, construido en Rust. El objetivo es proporcionar memoria persistente, buscable e interpretable con arquitectura de System 3 (recuperación, razonamiento, oversight).

## Restricciones y Decisiones

1. **Stack principal**: Rust, Tokio, Axum, SQLite + SQLite-vec
2. **Estado de largo plazo**: GitHub Issues en `.github/issues/`
3. **Estado de planificación/sesión**: `.gitcore/planning/PLANNING.md` y `.gitcore/planning/TASK.md`
4. **Interfaz primaria**: API HTTP + MCP transport
5. **CLI de ops**: Scripts en `scripts/` para Docker/health
6. **Arquitectura manda sobre ambigüedad**

## Servicios y Herramientas

| Capa | Herramienta | Uso |
|------|-------------|-----|
| Runtime | `cargo build --release` | Build producción |
| Tests | `cargo test -p xavier` | Validación |
| HTTP Server | `src/server/http.rs` | API + Panel UI |
| MCP | `src/server/mcp_server.rs` | Model Context Protocol |
| Docker | `docker-compose up` | Desarrollo local |
| Docs | `docs/site/` (Starlight) | Documentación pública |

## Fases

| Fase | Nombre | Objetivo | Estado |
|------|--------|----------|--------|
| F1 | Memoria Core | QMD + Belief Graph + Hybrid Search | ✅ Completado |
| F2 | MCP Integration | HTTP + MCP server para LLMs | ✅ Completado |
| F3 | Multi-tenant | Workspace isolation + quotas | ✅ Completado |
| F4 | Code Indexing | AST-backed symbol search | ✅ Completado |
| F5 | Production Ready | Deployment + Monitoring + Security | 🔄 En progreso |
| F6 | SQLite Optimization | SQLite + rtree extension para graph queries | ⏳ Pendiente |

## Criterios de Éxito

1. Todos los tests pasan (`cargo test -p xavier`)
2. API endpoints documentados en `docs/site/`
3. Multi-tenant isolation verificado
4. Docker deployment funcional
5. Panel UI accessible en `/panel`

## Riesgos y Mitigación

| Riesgo | Mitigación |
|--------|------------|
| Latencia >500ms en search | Optimizar RRF, considerar caching |
| SurrealDB inestable | SQLite/Vec como fallback default |
| Breaking changes en API | Versionar endpoints (`/v1/`) |
| Memory leaks en long-running | Monitorización + checkpoints |

---

*Xavier v0.4.1 - Cognitive Memory Runtime*
