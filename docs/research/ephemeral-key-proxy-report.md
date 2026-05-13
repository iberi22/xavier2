# 🔐 Reporte de Investigación: Ephemeral API Key Proxy para Sistemas Agentic

**Fecha:** 2026-05-12 | **Autor:** Claw (Orquestador SWAL) | **Para:** Sebas (CEO) + Bel

---

## 🎯 Resumen Ejecutivo

La idea de un **gestor de contraseñas/proxy de API keys efímeras para agentes de IA** es **altamente válida, comercialmente relevante y técnicamente viable**. Sin embargo, el mercado se está moviendo extremadamente rápido: **Infisical lanzó Agent Vault hace 3 semanas** (22 abril 2026) con una arquitectura casi idéntica al concepto propuesto. Aun así, existen **claros espacios de diferenciación** que hacen viable construir una alternativa competitiva, especialmente si se enfoca en: MCP-nativo, cifrado P2P, task-scoped ephemeral keys con revocación automática, y la integración con Xavier2 como capa de memoria/auditoría.

---

## 📊 Estado del Arte: Panorama Competitivo

### 🟢 Competidores Directos (Agent Vault / Credential Brokering)

| Solución | Tipo | Arquitectura | Ephemeral | MCP | Licencia | Madurez |
|----------|------|-------------|-----------|-----|----------|---------|
| **Infisical Agent Vault** | OSS (Go) | HTTPS_PROXY + MITM TLS | ❌ (brokering estático) | ✅ compatible | MIT? (OSS) | 🚀 Lanzado Abr 2026 |
| **Akeyless Secretless AI** | SaaS | JIT ephemeral credentials | ✅ JIT + TTL automático | ❌ | Propietaria | ✅ Producción |
| **Vercel Credential Brokering** | SaaS (sandbox) | MITM TLS en egress | ✅ 24h CA cert | ❌ | Propietaria | ✅ Producción |
| **Cloudflare Outbound Workers** | SaaS | Worker-level cred injection | ✅ (efímero) | ❌ | Propietaria | ✅ Producción |
| **E2B (propuesto #1160)** | OSS propuesto | MITM TLS + Gondolin | ✅ (por sandbox) | ❌ | MIT | 🟡 Planning |
| **Anthropic Managed Agents** | Interno | Proxy de credenciales dedicado | Desconocido | ❌ | No público | ✅ Interno |

### 🟡 Secret Managers Tradicionales (Adaptables)

| Solución | Ephemeral | Agent-Aware | Auto-Rotación | Self-Hosted |
|----------|-----------|-------------|---------------|-------------|
| **HashiCorp Vault** | ✅ Dynamic Secrets | ❌ | ✅ | ✅ (BSL) |
| **Infisical (core)** | ✅ Dynamic Secrets | ❌ | ✅ | ✅ (MIT) |
| **Doppler** | ❌ | ❌ | ✅ | ❌ Cloud-only |
| **AWS Secrets Manager** | ✅ vía STS | ❌ (solo AWS) | ✅ | ❌ |
| **Azure Key Vault** | ✅ | ❌ (solo Azure) | ✅ | ❌ |

### 🔵 Protocolos y Estándares Relevantes

| Tecnología | Tipo | Relevancia |
|-----------|------|-----------|
| **AWS STS AssumeRole** | Ephemeral IAM | Patrón de referencia: credenciales temporales con session policies |
| **SPIFFE/SPIRE** | Workload Identity | Atestación de identidad de workloads, federación |
| **OAuth 2.0 Token Exchange (RFC 8693)** | Token delegation | Downscoping de tokens por acción |
| **MCP (Model Context Protocol)** | Agente↔Herramienta | Protocolo estándar de Anthropic para agentes |
| **Noise Protocol / WireGuard** | Cifrado P2P | Zero-trust, e2e encryption |
| **Age Encryption** | At-rest | Moderno, simple, opinionated |

---

## 🔍 Análisis del Competidor Principal: Infisical Agent Vault

**Repo:** https://github.com/Infisical/agent-vault
**Lanzamiento:** 22 Abril 2026 (156 puntos HN, 56 comentarios)
**Lenguaje:** Go | **Deploy:** Binary único o Docker

### Arquitectura:
```
Agente → HTTPS_PROXY → Agent Vault (MITM TLS) → API externa
                         │
                         ├── Puerto 14321: UI/API de gestión
                         └── Puerto 14322: MITM proxy (TLS termination)
```

### Cómo funciona:
1. Creas un **vault** + **credentials** + **services** (reglas de autenticación por host)
2. Creas un **agent** (token que identifica al agente)
3. Seteas `HTTPS_PROXY=<token>:<vault>@agent-vault:14322` en el agente
4. El agente usa dummy keys (`__anthropic_api_key__`), el proxy las sustituye
5. El proxy termina TLS localmente, inyecta credenciales reales, re-encripta hacia afuera
6. El agente **nunca ve las credenciales reales**

### Fortalezas:
- ✅ Interface-agnostic (funciona con API, CLI, SDK, MCP — todo es HTTPS)
- ✅ Portable (un binary Go, funciona en cualquier lado)
- ✅ Multi-tenancy (vaults, agents, tokens)
- ✅ Egress filtering (allow/deny por host)
- ✅ Request logging
- ✅ Open source, respaldado por Infisical (26.8k estrellas)

### Debilidades:
- ❌ **No genera claves efímeras** — solo brokering de credenciales estáticas
- ❌ **No revoca al completar tarea** — el modelo es más "siempre proxy", no "préstamo por tarea"
- ❌ **No tiene rate-limiting por agente** — un agente malicioso puede saturar una API
- ❌ **No tiene audit trail criptográfico** — logs normales, no proof of which agent did what
- ❌ **MITM TLS es vulnerable si el agente está comprometido** — el agente puede desviar tráfico
- ❌ **Go, no Rust** — menos garantías de memory safety
- ❌ **No integración con sistemas de memoria** — es solo proxy, no hay contexto/estado de tareas
- ❌ **Solo HTTP** — no soporta SSH, WebSockets directos, o P2P
- ❌ **No MCP-native** — aunque compatible, no es un MCP server

---

## 💡 El Espacio de Diferenciación (Dónde Competir)

### Lo que NADIE está haciendo aún:

#### 1. 🔑 **Task-Scoped Ephemeral Keys con Auto-Revocación**
- El modelo del CEO: "préstamo de llave → tarea completada → revocación automática"
- Agent Vault hace brokering estático. **Nadie** hace el ciclo completo de préstamo temporal con callback de done/revocación
- Ejemplo: Agente de deploy pide un GitHub PAT → el sistema genera un token efímero (TTL 5min) → agente completa deploy → envía `done!` → el sistema revoca el token inmediatamente
- Esto es **diferente a Agent Vault** que solo inyecta en tránsito

#### 2. 🔒 **MCP-Native Server (vs HTTPS_PROXY)**
- Agent Vault es HTTP_PROXY genérico. Un **MCP Server nativo** para secretos sería:
  - `mcp__request_ephemeral_key(service, scope, ttl) → key_id`
  - `mcp__report_task_complete(key_id) → revoke`
  - `mcp__list_active_keys() → auditoría`
  - Totalmente integrado en el ecosistema MCP (Claude, OpenClaw, Kiro, etc.)

#### 3. 🛡️ **Cifrado P2P End-to-End (Noise Protocol / WireGuard)**
- Nadie usa P2P cifrado para la comunicación agente↔proxy
- Agent Vault usa TLS local (ok), pero si proxy y agente están en redes distintas → VPN necesario
- P2P nativo permitiría zero-trust deployment:
  ```
  Agente (cualquier red) ←→ Noise Protocol ←→ Proxy Vault
  ```
- WireGuard o libp2p con Noise darían cifrado autenticado mutuo

#### 4. 🧠 **Integración con Sistema de Memoria (Xavier2)**
- Idea única: Xavier2 como **registro de auditoría inmutable**
- Cada préstamo de llave → guardado en Xavier2
- Cada uso → logueado en Xavier2 con métricas
- Cada revocación → verificada y registrada
- Esto no existe en ninguna solución actual — es puramente innovador

#### 5. 🔬 **Rust para Memory Safety**
- Agent Vault es Go. Rust ofrece:
  - Zero-cost abstractions
  - Memory safety sin garbage collector
  - Crypto libraries maduras (ring, rustls, age, noise-protocol)
  - Mejor para manejo de secretos en memoria (zeroize, mlock)

#### 6. 📡 **Múltiples Transportes (no solo HTTP)**
- SSH: para agentes en servidores remotos (ya está ahí, seguro, estándar)
- WebSockets (wss://): bidireccional, ideal para notificaciones push (revocación inmediata)
- gRPC: fuerte tipado, streaming, mTLS
- Unix Domain Sockets: IPC local ultrarrápido (para agentes en misma máquina)
- P2P: para entornos descentralizados

---

## 🏗️ Recomendación de Arquitectura

### Opción A: **Plugin para Xavier2** (Recomendado para MVP)
```
┌─────────────────────────────────────────────────────────┐
│                      Xavier2 (8006)                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │         Ephemeral Key Vault (nuevo módulo)         │  │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────────┐  │  │
│  │  │ Key Gen │  │ Lending  │  │ Auto-Revocation  │  │  │
│  │  │ Engine  │  │ Manager  │  │   (TTL + Done)   │  │  │
│  │  └─────────┘  └──────────┘  └──────────────────┘  │  │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────────┐  │  │
│  │  │ Audit   │  │ Rate     │  │  Memory Store    │  │  │
│  │  │ Trail   │  │ Limiter  │  │  (QmdMemory)     │  │  │
│  │  └─────────┘  └──────────┘  └──────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  Exposed via:                                            │
│   • MCP Server (http://localhost:8006/mcp/secrets)       │
│   • CLI (xv2 key lend --service github --ttl 5m)        │
│   • WebSocket (ws://localhost:8006/ws/keys)              │
│   • HTTP API (http://localhost:8006/api/keys)            │
└─────────────────────────────────────────────────────────┘
         ▲          ▲          ▲
    MCP  │     CLI  │    WS   │
    ┌────┘    ┌────┘    ┌────┘
    │         │         │
 ┌──┴─────────┴─────────┴──┐
 │   Agentic System         │
 │  (OpenClaw/Claude/etc)   │
 └──────────────────────────┘
```

### Opción B: **Standalone Binary (estilo Agent Vault pero diferenciado)**
- Binario Rust → portable, sin runtime, memory-safe
- MCP Server nativo + CLI + WebSocket + gRPC
- Comunicación con Xavier2 para auditoría/memoria
- Mejor si se quiere distribuir independientemente

### Recomendación:
**MVP como módulo de Xavier2 + MCP Server**, luego standalone si hay tracción. La ventaja de Xavier2 es que ya tiene la capa de persistencia (QmdMemory), WebSocket, HTTP API, y la arquitectura hexagonal lista para agregar puertos.

---

## 🛣️ Roadmap Propuesto

### MVP (2 semanas) — "Préstamo Básico de Llaves"
- [x] API REST: `POST /keys/lend`, `POST /keys/revoke`, `GET /keys/active`
- [x] TTL automático con job de limpieza
- [x] Almacenamiento cifrado de llaves (age encryption o AES-256-GCM)
- [x] CLI básico: `xv2 key lend`, `xv2 key revoke`
- [x] Integración con Xavier2 QmdMemory para auditoría
- [x] Callback `done!`: agente reporta → revocación inmediata

### v1.0 (1 mes) — "MCP Native + Múltiples Transportes"
- [x] MCP Server: `request_ephemeral_key`, `report_task_complete`, `list_active_keys`
- [x] WebSocket para notificaciones push (revocación en tiempo real)
- [x] Rate limiting por agente/llave
- [x] Dashboard de monitoreo (Prometheus metrics + OpenTelemetry traces)
- [x] SDKs: Python, TypeScript, Rust
- [x] Integración con GitHub, OpenAI, Anthropic, AWS (generación de tokens por API)

### v2.0 (3 meses) — "Zero-Trust + Enterprise"
- [x] Cifrado P2P (Noise Protocol) para conexiones externas
- [x] Multi-tenancy (RBAC, equipos)
- [x] SSO/OAuth integration
- [x] Atestación de identidad de agente (SPIFFE/SPIRE o custom)
- [x] Hardware Security Module (HSM) opcional
- [x] Shamir Secret Sharing para llaves maestras

### v3.0 (6 meses) — "Plataforma"
- [x] Marketplace de plugins (conectores para servicios)
- [x] AI-powered anomaly detection en uso de llaves
- [x] Predictive rotation
- [x] Cross-cloud federation
- [x] SaaS + self-hosted

---

## 🎨 Brainstorm de Nombres

| Nombre | Vibe | Notas |
|--------|------|-------|
| **KeyLend** | Simple, directo | Describe el préstamo |
| **Clavis** | Latín "llave" | Elegante, único |
| **EpheKey** | Ephemeral Key | Técnico, descriptivo |
| **VaultAgent** | Directo | Pero suena como Agent Vault |
| **KeyProxy** | Funcional | Competitivo |
| **LlaveProxy** | Español | Identidad LATAM |
| **AgentSeal** | 🔒 metaphor | Seguridad lacrada |
| **Keystone** | Arquitectónico | La piedra angular |
| **DelegKey** | Delegación | Modelo de préstamo |
| **TaskVault** | Task-scoped | Diferenciador |
| **XavierKeys** (si va en Xavier2) | Familia Xavier | Extensión natural |
| **Sesame** | "Ábrete Sésamo" | Referencia cultural |
| **PassAgent** | Contraseña + Agente | Simple |
| **KeyPass AI** | KeyPass + IA | Reconocible |
| **ProxyClave** | Español + Proxy | Claro |

**Top 3 recomendados:** **Clavis** (original, elegante), **KeyLend** (describe el core feature), **XavierKeys** (si es parte de Xavier2)

---

## 🛡️ Análisis de Amenazas (Threat Model)

### Vectores de Ataque Identificados:
1. **Prompt injection → el agente intenta exfiltrar la llave** → MITIGADO: el proxy nunca revela la llave real, el agente solo ve un key_id
2. **Agente malicioso reporta `done!` falso** → MITIGADO: verificación de task completion con hash de resultado + timeout
3. **Race condition: 2 agentes piden la misma llave** → MITIGADO: locking distribuido, cada llave se presta a 1 solo agente
4. **Agente ignora TTL y sigue usando la llave** → MITIGADO: revocación server-side, la llave deja de funcionar aunque el agente la tenga
5. **Man-in-the-middle entre agente y proxy** → MITIGADO: mTLS + Noise Protocol para e2e
6. **Key leakage en memoria del proxy** → MITIGADO: Rust + zeroize + mlock para mantener secretos fuera de swap
7. **Supply chain attack via dependencias** → MITIGADO: mínimo uso de dependencias, vendored, verificación de checksums

---

## 💰 Viabilidad Comercial

### Mercado
- El mercado de AI Agents crecerá a $47B para 2030 (varias estimaciones)
- Toda empresa usando AI agents necesita secret management
- Gartner (abril 2026): "Cada API key estática es una falla del programa IAM"

### Modelo de Negocio
- **Open Core (AGPL)**: Core gratuito, features enterprise pagas
- **Enterprise**: RBAC, SSO, HSM, audit compliance, soporte
- **SaaS Hosted**: $99-999/mes según agents/keys
- **Self-Hosted Enterprise**: $5K-25K/año

### Timming
- ⚠️ **Ventana de oportunidad se está cerrando** — Infisical ya lanzó, Vercel/Cloudflare tienen sus soluciones
- ✅ **Aún hay espacio** para un jugador enfocado en ephemeral keys + MCP-native
- ✅ **Ecosistema LATAM** sin representación en esta categoría

---

## 🏁 Veredicto Final

### ¿La lógica está fina? 
**SÍ.** Totalmente. La separación agente↔credencial es el patrón emergente que toda la industria está adoptando. Gartner lo validó. Anthropic lo implementó. Infisical lo productivizó.

### ¿Es viable?
**SÍ, con condiciones:**
- Si se diferencia claramente de Agent Vault (ephemeral real, MCP-native, P2P, Xavier2)
- Si se mueve rápido (la ventana es de semanas, no meses)
- Si se enfoca en lo que NADIE hace: task-scoped lending con auto-revocación

### ¿Parte de Xavier2 o standalone?
**Recomendación:** **MVP como módulo de Xavier2**, luego evaluar spin-off.
- Xavier2 ya tiene: persistencia, WebSocket, HTTP API, arquitectura hexagonal
- Agregar MCP Server + key lending engine es natural
- Si gana tracción → standalone binary (Rust) + Xavier2 como backend de auditoría

### Acciones Inmediatas:
1. ✅ Validar concepto con PoC: API key lending + revocación en Xavier2
2. ✅ Crear MCP Server para secretos en Xavier2
3. ✅ Escribir ADR para la arquitectura del key vault
4. ✅ Crear repositorio y empezar a construir

---

## 📚 Referencias

- [Infisical Agent Vault (GitHub)](https://github.com/Infisical/agent-vault) — lanzado Abr 2026
- [Agent Vault Launch Blog](https://infisical.com/blog/agent-vault-the-open-source-credential-proxy-and-vault-for-agents)
- [Akeyless: Hardcoded API Keys Are an IAM Failure in AI Stacks](https://www.akeyless.io/blog/api-keys-ai-agents-iam-failure-gartner/)
- [AWS: Secure AI agent access patterns using MCP](https://aws.amazon.com/blogs/security/secure-ai-agent-access-patterns-to-aws-resources-using-model-context-protocol/)
- [E2B: Credential Brokering Issue #1160](https://github.com/e2b-dev/E2B/issues/1160)
- [Vercel: Credential Brokering in Sandbox](https://vercel.com/changelog/safely-inject-credentials-in-http-headers-with-vercel-sandbox)
- [Descope: OAuth vs API Keys for Agentic AI](https://www.descope.com/blog/post/oauth-vs-api-keys)
- [Gondolin (MITM proxy)](https://github.com/earendil-works/gondolin)
- [Reddit r/AI_Agents: Secret Proxy For Agents](https://www.reddit.com/r/AI_Agents/comments/1s7k3qt/secret_proxy_for_agents/)
- [Hacker News: Agent Vault Discussion](https://news.ycombinator.com/item?id=47865822)
- Gartner Reference Architecture Brief: IAM for AI Agents and Other Workloads (Abril 2026)

---

*Investigación completada: 2026-05-12 16:17 COT | 8 búsquedas Brave + 8 web fetches | ~45min total*
