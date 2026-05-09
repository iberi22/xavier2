Quiero integrar un modulo create en rust en mi entrono, este modulo se ecnargara de arministrar los secretos ssh leys, y contraseñas del sistema. quuieri que sea un binario instalable en multiples dispositivos, con la capacidad de tener sincronizacion por medio de red P2P en una red privada con cifrado pos cuantico, sencilla de usar, minimalista, sin backend sin servidores, la idea es poder tener una app en el telefono y en mi pc windows y en mi macbook que sincronice mis contraseñas de forma segfura entre los dispositivos, quiero que la hagas en rust con la ultima tegonologia y logicas como las de https://github.com/openbao/openbao, y otros repos, primero investiga y realiza un plan con todo el enfoque e idemas, construye el plan de implementacion por fases, presentamelo para aprobarlo

Voy a investigarlo como un producto serio de seguridad en Rust: revisaré OpenBao, P2P privado, cifrado poscuántico, almacenamiento seguro y compatibilidad Windows/macOS/móvil, y luego te entregaré una arquitectura objetivo con decisiones justificadas, riesgos y un plan por fases para tu aprobación.

Ya tengo una dirección bastante clara: conviene tratar OpenBao como referencia de principios —seal/unseal, barrera criptográfica, rotación, auditoría y separación de capas— pero no copiar su modelo servidor-centralizado. Para tu caso encaja mejor un vault local-first con P2P sobre QUIC, empaquetado con Tauri 2, y cifrado híbrido clásico+poscuántico montado por encima del transporte; ahora estoy cerrando la elección entre iroh y libp2p y cómo resolver la conectividad móvil sin traicionar el objetivo de “sin backend”.

Ya cerré la restricción clave: en Internet abierto y móviles no es realista prometer P2P puro siempre activo; el plan será sincronización oportunista sobre una red privada por defecto, con relay autoalojado opcional.

Thought for 26m 23s

Sí es viable, pero no te recomendaría construir un “OpenBao en miniatura”. Te recomendaría construir un vault local-first, multi-dispositivo, P2P directo sobre red privada, con un núcleo Rust compartido y apps ligeras encima. De OpenBao tomaría cuatro ideas: barrier encryption sobre almacenamiento no confiable, seal/unseal, auditoría, y revocación/rotación; no copiaría su modelo de servidor HTTP, secrets engines y ACLs multiusuario, porque OpenBao está pensado para un sistema server-side con backend de almacenamiento y operación centralizada.

Lo más útil de OpenBao para tu caso es esto: su barrera cifra todo lo que sale hacia un backend considerado no confiable; su estado sealed/unsealed separa claramente “tengo el almacenamiento” de “puedo descifrar el almacenamiento”; y su modelo de lease/revoke fuerza rotación y revocación explícita. Traducido a tu producto: base local no confiable + root key local + vault keys por bóveda + revocación de dispositivos + log de auditoría tamper-evident.

La parte delicada es la promesa de “cifrado poscuántico” completa. Hoy sí puedes desplegar intercambio de claves híbrido ML-KEM + clásico en Rustls; el soporte de ML-KEM ya se movió al crate principal de rustls, rustls recomienda rustls-aws-lc-rs, y aws-lc-rs build/testea en Windows, macOS, iOS y Android. Pero incluso PQXDH de Signal dice explícitamente que, en su revisión actual, la autenticación mutua sigue dependiendo del logaritmo discreto y que la autenticación mutua deniable totalmente poscuántica sigue siendo un problema abierto. Así que la promesa honesta para v1 no debe ser “todo el sistema es 100% PQ en todas sus propiedades”, sino “transporte híbrido PQ y resistencia HNDL en el establecimiento de sesión”.

Para el estado multi-dispositivo, la tecnología correcta sí me parece MLS. RFC 9420 está diseñado justo para establecimiento asíncrono de claves de grupo con forward secrecy y post-compromise security, y OpenMLS lo implementa en Rust. Además, OpenMLS ya añadió un modo PQ experimental con X-Wing + ML-KEM, pero su propio equipo aclara que ese ciphersuite todavía no tiene codepoint IANA y que la interoperabilidad no está garantizada. Mi lectura: usar MLS desde el principio, sí; usar el modo PQ completo de MLS, sólo detrás de feature flag hasta que madure.

Tu requisito de “sin backend, sin servidores” cambia toda la arquitectura de red. Si quisieras P2P por internet entre NATs, opciones como iroh o libp2p recurren a hole punching y, cuando eso no basta, a relays/circuit relays. Como tú pediste explícitamente cero servidor, el MVP debe limitarse a LAN o red privada preexistente (por ejemplo una VPN/overlay que ya controles), con conexión QUIC directa y sin relay. Eso sí es coherente con el requisito y además simplifica mucho la seguridad.

En móvil también conviene ser muy realista: iOS exige permiso para red local y usa BackgroundTasks para trabajo en segundo plano; Android ofrece NSD para descubrimiento y WorkManager para tareas persistentes. En la práctica, el objetivo razonable de la primera versión es sync al abrir, al emparejar, al volver a foreground y por refresco manual/opotunista, no un daemon perfecto siempre vivo.

Tomaría de age la filosofía de UX: claves explícitas, cero configuración y minimalismo. El repo oficial de age ya soporta claves híbridas PQ en su implementación Go. Pero no usaría el crate Rust age como núcleo del producto: su documentación sigue marcándolo pre-1.0/beta, y además su soporte de SSH existe por compatibilidad; la propia doc recomienda preferir claves nativas X25519 antes que reutilizar claves SSH para cifrado. Para material SSH usaría ssh-key, y para compatibilidad/importación KDBX usaría keepass-rs, pero no usaría KDBX como formato nativo porque la escritura KDBX4 en ese crate sigue marcada como experimental. KeePassXC te sirve mejor como benchmark funcional: KDBX, SSH agent, browser integration, passkeys, TOTP.

También descartaría basarme en liboqs o en el crate puro ml-kem para el camino principal del MVP. liboqs se presenta como biblioteca para prototipado y experimentación de algoritmos quantum-safe, y el crate ml-kem avisa explícitamente que no ha sido auditado de forma independiente. Para un producto multi-plataforma y minimalista prefiero que el camino PQ principal esté en Rustls/aws-lc-rs, y dejar KEMs “raw” sólo para módulos experimentales o de laboratorio.

Arquitectura que te recomiendo

Núcleo Rust compartido. Un workspace con vault-core, vault-crypto, vault-store, vault-sync, vault-agent, vault-cli y vault-app. Toda la lógica sensible vive en Rust; la UI es una capa delgada. Para compartir lógica entre plataformas, Tauri v2 ya soporta desktop y mobile desde un solo codebase, y UniFFI te deja dejar una salida limpia hacia shells nativos Swift/Kotlin si después quieres endurecer más el móvil.

Criptografía local. Haría una jerarquía de claves inspirada en OpenBao: passphrase maestra → KEK derivada con Argon2id → root key aleatoria → vault keys → data keys por item/adjunto. Las claves sensibles irían envueltas en secrecy y zeroize para reducir fugas accidentales en memoria. El almacenamiento local sería “no confiable”: el contenido real de cada item se guarda ya cifrado e íntegro antes de tocar disco.

Almacenamiento. Usaría SQLite como índice local y metadatos, pero con los blobs sensibles cifrados por la capa de aplicación; así evitas meter SQLCipher/C extra en la primera iteración y mantienes la semántica de “barrier sobre backend no confiable”. rusqlite es suficiente para eso.

Red y confianza entre dispositivos. Transporte sobre QUIC con quinn + rustls, autenticación mutua por certificado/clave pública pinneada, y prefer-post-quantum para el handshake híbrido cuando el proveedor lo permita. Para la identidad de dispositivos, usaría huellas precompartidas y pinning out-of-band al estilo Syncthing: el propio protocolo BEP documenta autenticación basada en el certificado TLS y huellas/certificate pinning como Device IDs.

Sincronización. No metería el vault entero dentro de un CRDT genérico en v1. Haría un append-only signed event log con snapshots y compactación: operaciones como create_item, update_field, rotate_key, delete_item, add_device, revoke_device. La sincronización es incremental; el merge es de dominio y auditable. Para la seguridad de grupo y rekeying usaría MLS, y al revocar un dispositivo rotaría epoch y claves de vault. Automerge es una referencia útil de “local-first/offline-first”, pero para un gestor de secretos prefiero un log auditable y más fácil de revisar.

Apps. UI con Tauri v2 en desktop y mobile, pero con un trait claro DeviceKeyStore por plataforma: Apple Keychain en macOS/iOS, Android Keystore en Android y DPAPI/Credential Manager en Windows. En Android y Apple el almacenamiento de claves del sistema está pensado justamente para mantener el material no exportable o dentro de un contenedor seguro del SO.

SSH y compatibilidad. Soportaría desde temprano: importar claves OpenSSH, generar Ed25519, proteger passphrases, exportar públicas, y más adelante integrar con ssh-agent/Pageant como hace KeePassXC, en lugar de intentar reemplazar el agente del sistema desde el día uno.

Plan de implementación por fases

Fase 0 — RFCs y modelo de amenaza
Cerrar cuatro ADRs: red privada sin relay, QUIC + rustls híbrido PQ, MLS para membresía/rekey, y formato nativo propio con importación KDBX/SSH. Aquí también dejamos escrito qué está fuera de alcance: dispositivo comprometido y desbloqueado, host con ejecución maliciosa y análisis de memoria en caliente, igual que OpenBao marca esos escenarios como fuera de su threat model.

Fase 1 — Núcleo local y CLI
Entregables: vault init, vault unlock, CRUD de credenciales y claves SSH, jerarquía de claves, auditoría local hash-chain, almacenamiento local cifrado, adapters de secure storage para Windows y macOS primero. Resultado esperado: el fichero/DB robado no revela secretos, y puedes usar el binario desde terminal en Windows/macOS.

Fase 2 — Identidad de dispositivo y emparejamiento
Entregables: device keypair, QR invite, fingerprint verification, pinning del peer, descubrimiento LAN y conexión QUIC directa en LAN/VPN privada. Resultado esperado: dos dispositivos se emparejan y sincronizan un snapshot completo sin servidor.

Fase 3 — Sync incremental y revocación
Entregables: event log incremental, snapshots, compactación, manejo de conflictos por tipo de item, MLS para membresía y rekey, revocación de dispositivo y rotación de epoch/vault keys. Resultado esperado: tres dispositivos convergen tras ediciones offline/online, y un dispositivo revocado deja de poder leer datos nuevos.

Fase 4 — Apps de escritorio y móvil
Entregables: app desktop, app móvil, desbloqueo biométrico cuando el sistema lo permita, flujo de permisos de red local, sync en foreground y background oportunista. Resultado esperado: experiencia usable y minimalista en Windows, macOS y móvil, sin prometer un sync “siempre vivo” que el SO no garantiza.

Fase 5 — SSH agent, import/export y recovery
Entregables: integración con ssh-agent/Pageant, importación KDBX/SSH, export controlado, recovery kit opcional con shares tipo Shamir para usuarios avanzados, y políticas de “panic seal” local para bloquear rápido la bóveda si detectas compromiso.

Fase 6 — Hardening y distribución
Entregables: fuzzing, revisión externa de crypto/protocolo, cargo-deny, cargo-vet, cargo-auditable, builds con cargo-dist y firma/verificación de artefactos con Cosign. Resultado esperado: binarios auditables y firmados para distribuir con bastante confianza en Windows y macOS, y pipeline limpio para móvil.

Lo que aprobaría yo como alcance v1

Aprobaría estas cuatro decisiones como base:

Sync sólo en LAN/VPN/red privada, sin relays ni backend.

Transporte híbrido PQ ya, pero PQ MLS completo sólo experimental.

Formato nativo propio, con import/export KDBX y SSH; no KDBX como storage principal.

Core Rust primero, UI compartida encima; desktop antes de endurecer móvil al 100%.

Con tu visto bueno sobre esas cuatro decisiones, el siguiente paso natural es bajar esto a un RFC técnico detallado con layout del workspace, contratos entre crates y protocolo de emparejamiento.

mejora el plan para que esta logica de keys corra bajo una capa agentic, es decir quiero que uses mi proyecto para que esta capa de administracion de keys pueda ser desacoplable y vivir como una dependencia para instalar en otros sistemas como paquete de cargo tambien. deberia de poder manejar tambien los tokens de logueo de los navegadores para poder tener una capa de administracion de credenciales de navegador pudiendo este sistema pueda intercambiar entre varios tokens de sessiones de navegacion para asi eviar hacer logueos, solo cambiando el token o la cokie, deberiamos de tener en esta capa un sistema de seguridad que tenga una capa que no permita que se puedan robar estas credenciales de los navegadores facilmente. ya que deberiamos de menejarlas de forma tokenisada y aislada investiga la mejor manera de tener todo esto, crea un baglog robusto planiint, roadmap

Voy a rehacer el plan con dos frentes: un núcleo Rust desacoplable tipo crate/agent y un diseño seguro para credenciales web, distinguiendo lo viable de lo riesgoso.

Ya confirmé el giro del diseño: el núcleo puede vivir como crate + agente local de capacidades con políticas, auditoría y aislamiento, pero no haré portabilidad de cookies/sesiones. En navegador, la vía segura será extensión + native messaging + broker local, con mediación explícita del usuario y foco en credenciales delegadas o passkeys.

Thought for 16m 6s

Sí, pero haría un cambio fuerte de alcance: no planearía un subsistema para capturar, intercambiar o reutilizar cookies/tokens bearer de sesiones de sitios de terceros entre dispositivos. Eso se parece demasiado a portabilidad de sesión fuera del control del proveedor. Las sesiones web son el vínculo entre autenticación y autorización, las APIs de cookies requieren permisos explícitos por host, y además ya existen cookies particionadas que vuelven ese enfoque más frágil y riesgoso. En cambio, sí diseñaría una capa de credenciales de navegador segura con mediación explícita del usuario, aislamiento local, y un broker de sesiones sólo para servicios propios o integraciones que soporten OAuth token exchange y sender-constrained tokens.

Enfoque revisado

Lo replantearía en tres capas:

Core embebible: biblioteca Rust publicable en crates.io, sin UI, sin daemon obligatorio.

Agent local: proceso opcional residente que expone IPC local y encapsula políticas, caché, sealing, auditoría y sync.

Clients: CLI, desktop/mobile y extensión de navegador, todos consumiendo el mismo SDK o el mismo protocolo local.

La inspiración correcta de OpenBao no es copiar su servidor, sino heredar sus patrones: barrier encryption sobre storage no confiable, seal/unseal, audit devices, response wrapping y la idea de agent como daemon cliente con auto-auth, proxy y caché. Eso encaja muy bien con tu requisito de una lógica desacoplable que también pueda vivir como dependencia o como servicio local.

Qué significa “agentic” aquí

No metería un LLM dentro del perímetro de confianza. Aquí “agentic” debe significar runtime autónomo pero determinista, guiado por políticas y eventos: emparejar dispositivos, rotar claves, sellar por riesgo, emitir grants temporales, sincronizar, auditar y resolver flujos de aprobación. Si más adelante quieres una capa IA, la pondría fuera del TCB, operando sólo sobre metadatos redactados y handles opacos, nunca sobre secretos crudos.

Arquitectura objetivo

Usaría un workspace Cargo con crates separados. Cargo soporta workspaces con miembros, workspace.package, workspace.dependencies y workspace.lints, así que puedes publicar varias piezas y compartir versión, dependencias y políticas del repo. Para móvil/desktop, Tauri v2 ya soporta Windows, macOS, Android e iOS desde un mismo codebase; y UniFFI te deja exponer el núcleo Rust a Swift/Kotlin si después quieres shells nativos más duros.

workspace/
  crates/
    vault-core              # modelos, tipos, invariantes
    vault-crypto            # jerarquía de claves, wrap/unwrap, KDF, AEAD
    vault-store             # SQLite + blobs cifrados + snapshots
    vault-sync              # QUIC + MLS + pairing + revocación
    vault-agent-proto       # tipos IPC/RPC/capabilities/events
    vault-agent             # daemon local / service
    vault-sdk               # API pública para Rust
    vault-browser-broker    # políticas y conectores de navegador
    vault-platform          # Keychain/Keystore/DPAPI adapters
    vault-ffi               # UniFFI bindings Swift/Kotlin
    vault-cli               # binario installable
  apps/
    desktop
    mobile
  extensions/
    browser-webext
Contratos públicos que sí publicaría

vault-sdk: crate principal para que otros proyectos Rust lo consuman.

vault-agent-proto: tipos y contratos estables entre clientes y daemon.

vault-browser-broker: capa opcional para extensiones o integraciones locales.

vault-cli y vault-agent: binarios separados.

La regla sería: todo lo sensible nace en vault-core/vault-crypto; todo lo “agentic” depende de eso, no al revés.

Diseño de seguridad revisado

Para la base local, mantendría el patrón tipo OpenBao: storage local tratado como no confiable; root key y keyring separados; estado sealed/unsealed; y auditoría encadenada. Para endurecer memoria, secrecy hace explícito el acceso a secretos y evita fugas accidentales por debug/logging, mientras zeroize ayuda a borrar memoria de forma fiable; para claves SSH, ssh-key ya cubre formatos OpenSSH y material relacionado.

Para sync privado, mantendría QUIC + rustls. quinn es una implementación QUIC en Rust; rustls recomienda rustls-aws-lc-rs cuando necesitas el set más completo de features, incluyendo post-quantum algorithms; y MLS sigue siendo la mejor base para membresía de grupo, forward secrecy y post-compromise security. Para PQ en MLS, lo dejaría experimental, porque OpenMLS sí tiene modo con X-Wing + ML-KEM, pero su propia documentación avisa que aún no tiene codepoint IANA y que la interoperabilidad no está garantizada.

Capa de credenciales de navegador: lo que sí haría

La construiría como Browser Credential Mediation Layer, no como “cookie manager”.

Soporte permitido en MVP

Passwords, API tokens, TOTP, claves SSH y metadatos de cuenta.

Asistencia de login por extensión con consentimiento del usuario.

Broker local por native messaging entre extensión y agente nativo.

Políticas por origen y allowlist explícita.

Autofill / approval flows / reauth flows.

Sesiones sólo para aplicaciones o dominios que tú controles, usando token exchange o reemisión segura, no copiando cookies.

Chrome permite a la extensión hablar con una aplicación nativa registrada en un proceso separado mediante native messaging; las cookies requieren permiso cookies y host permissions por dominio. Además, Chrome recomienda no guardar datos sensibles en storage.local o storage.sync, y usar storage.session para datos sensibles en memoria.

Soporte que dejaría fuera

Exportar cookies crudas de terceros.

Importar cookies crudas entre dispositivos.

“Cambiar token/cookie y entrar sin login” para servicios ajenos.

Cualquier flujo que transforme una cookie bearer de un tercero en un artefacto portable.

Alternativa segura para “sesión sin relogueo”

Para tus propios servicios, haría un Session Broker:

el agente guarda refresh tokens o material de sesión en el almacén seguro local;

emite grants opacos de corta vida;

el cliente redime el grant para obtener un token nuevo;

el token va sender-constrained con DPoP cuando el backend lo soporte;

si hay revocación o riesgo, el grant expira o el agente se sella.

DPoP está justamente pensado para sender-constrain OAuth tokens y detectar replay; y RFC 8693 define token exchange para pedir/obtener nuevos tokens desde el authorization server.

Passkeys y credenciales de navegador

Passkeys/WebAuthn no deberían tratarse como “otro secreto exportable”. WebAuthn define credenciales de clave pública scoped a un relying party, accesibles por el user agent con mediación y consentimiento del usuario. La dirección correcta es interoperabilidad de proveedor de credenciales, no scraping del storage del navegador. FIDO ya publicó CXP y CXF para intercambio seguro de credenciales entre providers, incluyendo contextos online/offline, pero CXF sigue marcado como Working Draft y explícitamente dice que no debe tomarse todavía como base estable de implementación. Por eso lo pondría como pista experimental, no como pilar del MVP.

Cómo tokenizar y aislar la capa browser

Aquí sí haría algo fuerte:

El navegador no recibe secretos maestros. Recibe CapabilityHandles o WrappedGrants con TTL muy corto.

El agente conserva el secreto real en memoria protegida y/o almacenamiento del SO.

Los grants son single-use al estilo response wrapping de OpenBao: el cliente recibe un artefacto efímero que debe “unwrapear” una sola vez.

Nada sensible persistente en chrome.storage.sync ni local.

El extension host no tiene wildcard global; permisos por dominio y consentimiento por acción.

Perfiles y sesiones first-party aislados por origen, dispositivo y policy.

Re-seal automático por idle, lock screen, cambio de red, o señal de riesgo.

Auditoría local con hash/HMAC de campos sensibles y múltiples sinks lógicos, inspirado en OpenBao.

Secure storage por plataforma

La clave de envoltura del vault y las device keys deben vivir en los almacenes del SO:

Apple Keychain está pensado para pequeños secretos y claves en una base cifrada.

Android Keystore permite que una app guarde credenciales que sólo esa app puede usar.

Windows DPAPI cifra datos atados al usuario o a la máquina.

Backlog robusto
P0 — Fundaciones y límites de seguridad

ADR-001: alcance del browser layer
Definir formalmente que no se sincronizan cookies/sesiones de terceros.
DoD: documento de alcance, threat model y “out of scope”.

ADR-002: agentic runtime
Definir que el agente es policy-driven y que cualquier IA queda fuera del TCB.
DoD: capability model, approval model, surface redacted.

ADR-003: workspace público
Separar crates publicables (vault-sdk, vault-agent-proto) de crates internos.
DoD: cargo publish --dry-run limpio para crates públicos.

ADR-004: modelo de credencial
Tipos nativos: Password, ApiToken, SshKey, TotpSeed, PasskeyMeta, FirstPartySessionHandle.
DoD: schemas versionados y compatibilidad forward/backward.

P1 — Núcleo criptográfico y sealing

Jerarquía de claves
Passphrase/KDF → KEK → root key → vault keys → item keys.
DoD: test vectors, rotación y recuperación.

Barrier storage local
SQLite para índice + blobs cifrados por aplicación.
DoD: robo del DB no revela material útil.

Seal / unseal / panic seal
Estado sealed, unlock explícito, re-seal por idle/risk.
DoD: al sellar, memoria sensible y grants activos se invalidan. Inspiración directa de OpenBao.

Higiene de memoria
secrecy + zeroize, límites estrictos de logging/debug.
DoD: secretos no serializables por defecto y tests anti-leak.

P1 — Agente local desacoplado

IPC local estable
Unix domain sockets / named pipes / loopback local según plataforma.
DoD: SDK y CLI consumen el mismo protocolo.

Capability engine
ReadMasked, RevealOnce, UseForSshSign, FillPassword, MintSessionGrant, RotateKey, PairDevice.
DoD: toda operación sensible requiere capability + policy.

Response wrapping local
Artefactos single-use y TTL corto para entrega segura entre agente y cliente.
DoD: grants reutilizados fallan.

Auditoría
Log append-only con hash-chain, eventos correlacionables y campos sensibles hasheados.
DoD: verificación offline de integridad y export de auditoría. Inspirado en audit devices de OpenBao.

P1 — Sync multi-dispositivo

Identidad de dispositivo y pairing
Device keys, fingerprints, QR invite, pinning.
DoD: dos dispositivos se emparejan sin servidor.

Transporte P2P privado
QUIC con rustls; PQ híbrido donde el provider lo permita.
DoD: sync funcional en red privada/VPN sin relay.

Membresía y rekey
MLS para conjunto de dispositivos, con PQ MLS bajo feature experimental.
DoD: revocación de dispositivo fuerza nueva epoch y bloquea lectura futura.

Event log + snapshots
Operaciones de dominio, no CRDT genérico.
DoD: convergencia entre 3 dispositivos y compactación consistente.

P2 — Browser Credential Mediation Layer

Extensión + native host
Extensión WebExtensions + host nativo local.
DoD: handshake autenticado extensión↔agente y permisos por origen.

Storage seguro del lado browser
Sólo estado efímero en memoria (storage.session) y trusted contexts.
DoD: cero secretos persistentes en sync/local.

Password/TOTP assist
Rellenado con consentimiento y policy por origen.
DoD: no hay export masivo; sólo uso puntual.

Passkey mediation
Guardar metadatos, estado y recovery UX; no exfiltrar ni “raw-exportar” credenciales WebAuthn.
DoD: compatible con enfoque origin-bound y user-mediated.

Cookies/sessions third-party: deny by design
Regla explícita en policy engine.
DoD: el sistema rechaza intents de export/import raw session material.

P2 — Session Broker first-party

Conectores OAuth propios
Sólo para dominios que tú controles.
DoD: el agente renueva/migra sesión sin copiar cookies.

Token exchange
RFC 8693 para emitir nuevos tokens por device/app context.
DoD: sesión en nuevo dispositivo vía backend propio, no vía bearer replay.

Sender-constrained tokens
DPoP o mecanismo equivalente donde aplique.
DoD: token robado fuera del binding no sirve.

P3 — Apps, SDK y distribución

CLI y daemon productivos
vault-cli y vault-agent como binarios instalables.

SDK Rust público
API estable, docs, ejemplos y features opt-in.

Desktop/mobile
Tauri v2 para shell compartido o UniFFI + shell nativo cuando toque endurecer.

Publicación y semver
Versionado por crates, changelog, compat matrix y policy de breaking changes.

P3 — Research track

CXP/CXF experimental
Seguimiento de drafts de FIDO.
DoD: spike aislado, no activado por defecto.

PQ MLS GA decision
Sólo pasar a GA cuando interoperabilidad y estandarización maduren.

Roadmap por fases

Fase 1 — Library-first
Publicar vault-core, vault-crypto, vault-sdk, vault-cli. Sin navegador todavía. Objetivo: vault local, seal/unseal, SSH/passwords/TOTP, auditoría.

Fase 2 — Agent-first
Añadir vault-agent, IPC, capabilities, response wrapping, secure storage por plataforma. Objetivo: que cualquier app Rust pueda usar la dependencia o el daemon indistintamente.

Fase 3 — P2P sync
QUIC + pairing + MLS + revocación. Objetivo: desktop y móvil sincronizan secretos en red privada.

Fase 4 — Browser safe MVP
Extensión + native host + password/TOTP assist + passkey metadata + allowlists. Objetivo: UX de navegador sin persistencia insegura ni cookies portables.

Fase 5 — First-party session broker
Integraciones con tus propios servicios usando token exchange/DPoP. Objetivo: “no relogueo” legítimo en tus dominios, sin bearer replay.

Fase 6 — Hardening y ecosistema
Docs, publishing, fuzzing, auditoría externa, soporte enterprise policies, recovery kit y research de CXP/CXF.

Decisiones que te pediría aprobar

Queda fuera del producto la exportación/sync de cookies o tokens de sesión de terceros.

La capa “agentic” será daemon local + SDK embebible, no un servidor central.

La portabilidad de sesión sólo existirá para servicios propios, vía backend controlado y tokens sender-constrained.

Passkeys se tratan como credenciales origin-bound; su intercambio quedará en pista experimental hasta que CXP/CXF maduren.

El perímetro de confianza no incluirá IA con acceso a secretos crudos.

Si apruebas estas cinco decisiones, el siguiente paso lógico es bajar esto a un RFC técnico de workspace + contratos de crates + policy model del agent.
