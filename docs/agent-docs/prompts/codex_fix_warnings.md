Arregla los siguientes problemas WARNING identificados en el code review:

## 1. API keys debiles (src/security/auth.rs:75)
La funcion User::new() genera api_key con ulid::Ulid::new().to_string().
Cambialo a: genera 32 bytes aleatorios con OsRng, codifica en hex (64 chars).
Necesitas: use rand::rngs::OsRng; use rand::RngCore; en el archivo.
Si rand no esta en Cargo.toml como dependencia del crate raiz, agregala.

## 2. Errores silenciados (src/adapters/inbound/http/routes.rs)
Los .set(...).ok() de TIME_STORE y HEALTH_PORT descartan errores de inicializacion.
Cambialos a .set(...).expect(...) con un mensaje claro que indique que global fallo.

## 3. Arquitectura hexagonal - DIP violado en app/
Los archivos en src/app/ importan infraestructura concreta en vez de ports.
- src/app/qmd_memory_adapter.rs: importa crate::memory::qmd_memory y crate::memory::schema
- src/app/security_service.rs: importa crate::security
NO los muevas ahora. Solo agrega un comentario // TODO: HexArch - depends on concrete infra, should use port abstraction al inicio de cada archivo.

## 4. serve() stub en src/server/http.rs
HttpServer::serve() solo duerme 50ms y retorna.
Agrega un tracing::warn!() diciendo "HttpServer::serve() is a stub - does not actually start a server"

## 5. Package name incorrecto en Cargo.toml
Cambia name = "xavier2-1" a name = "xavier2"

Para cada cambio: compila con cargo build --lib y verifica que pase.
No modifiques nada que no este en esta lista.
