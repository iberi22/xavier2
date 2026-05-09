$body = @{
    path = "company/swal/business-lines"
    content = "# Líneas de Negocio - SWAL\n\n1. **Desarrollo de Software** - Rust, TypeScript, automatización\n2. **Creación de Contenido** - Producción, automatización\n3. **Trading de Criptoactivos** - Copy trading, estrategias\n4. **Servicios SEO** - Agentes IA para optimización de motores de búsqueda\n5. **Agentes IA Alquilables** - Agentes con memoria infinita para clientes\n6. **Finetuning de modelos** - modelos de lenguaje, modelos de visión\n7. **Investigación** - investigación, análisis de datos\n8. **Seguridad Informática** - análisis de vulnerabilidades, penetración"
    metadata = @{type = "company"; updated = "2026-04-13"}
} | ConvertTo-Json -Compress

$temp = [System.IO.Path]::GetTempFileName() + ".json"
[System.IO.File]::WriteAllText($temp, $body)
$TOKEN = $env:XAVIER_TOKEN
if (-not $TOKEN) { $TOKEN = $env:XAVIER_API_KEY }
if (-not $TOKEN) { $TOKEN = $env:XAVIER_TOKEN }
if (-not $TOKEN) {
    throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
}
curl.exe -s -X POST "http://localhost:8003/memory/add" -H "X-Xavier-Token: $TOKEN" -H "Content-Type: application/json" --data-binary "@$temp"
Remove-Item $temp
