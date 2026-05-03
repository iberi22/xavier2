Arregla estos problemas pendientes del code review:

## 1. Reemplazar unwrap/expect runtime peligrosos en auth.rs
En src/security/auth.rs hay 4 unwrap() en SystemTime::duration_since(UNIX_EPOCH):
- Linea 22: Claims::new()
- Linea 37: Claims::is_expired()
- Linea 74: User::new()
- Linea 226: somewhere else
Cambialos a .expect("SystemTime::duration_since failed - clock is before UNIX epoch"). Esto hace panic pero con mensaje claro en vez de unwrap generico. No cambies la logica, solo el mensaje.

## 2. Feature-gate ratatui y crossterm
En Cargo.toml, mueve ratatui y crossterm detras de un feature flag "cli-interactive".
Agrega:
[features]
default = ["cli-interactive"]
cli-interactive = ["ratatui", "crossterm"]
Y marca ratatui y crossterm como dependencias opcionales.
En src/cli.rs, cualquier codigo que use ratatui o crossterm ponlo detras de #[cfg(feature = "cli-interactive")].

## 3. Limpiar allow(dead_code) falsos
Encuentra #[allow(dead_code)] que ocultan warnings reales. Para cada uno:
- Si es legitimo (struct usado en tests pero no prod), deja el allow
- Si es deuda real (codigo muerto que nadie usa), agrega un comentario TODO: Dead code - remove or implement

## 4. Dependencias opcionales
Si hay crates que solo se usan en tests (como rstest, testcontainers), marcalos como [dev-dependencies] en Cargo.toml.

Para cada cambio: compila con cargo build --lib y verifica que pase.
NO modifiques nada que no este en esta lista.
