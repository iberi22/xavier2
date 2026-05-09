# Xavier HTTP API

Base URL:

```text
http://localhost:8003
```

Auth header:

```text
X-Xavier-Token: <token>
```

Implemented routes:

- `GET /health`
- `POST /memory/add`
- `POST /memory/delete`
- `POST /memory/reset`
- `POST /memory/search`
- `POST /memory/query`
- `GET /memory/graph`
- `POST /agents/run`
- `POST /sync`
- `POST /code/scan`
- `POST /code/find`
- `GET /code/stats`

Payload shapes:

```json
{"content":"text","path":"optional/path","metadata":{"any":"json"}}
```

```json
{"query":"search terms","limit":10}
```

```json
{"id":"optional-id","path":"optional/path"}
```

```json
{"path":"E:/scripts-python/xavier"}
```

```json
{"query":"AgentRuntime","limit":10,"kind":"struct","pattern":"optional-pattern"}
```
