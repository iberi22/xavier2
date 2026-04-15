@{
    path = "company/swal/overview"
    content = "# SouthWest AI Labs\n\n**Website:** https://github.com/southwest-ai-labs\n**Proyectos:** E:\\scripts-python\n**GitHub Repo Principal:** iberi22/* (NO southwest-ai-labs)\n**Infraestructura:** Proxmox (192.168.1.10) - apagada por ahorro\n\n## Líneas de Negocio\n1. Desarrollo de Software - Rust, TypeScript, automatización\n2. Creación de Contenido - Producción, automatización\n3. Trading de Criptoactivos - Copy trading, estrategias\n4. Agentes IA Alquilables - SEO, trading, desarrollo, contenido, segurança\n5. Finetuning de modelos - lenguaje, visión\n6. Investigación - análise de datos\n7. Seguridad Informática - vulnerabilidades, penetración\n\n## Filosofía\n> La comodidad de todos los seres humanos en la Tierra, brindando tecnología segura, abierta e inteligente."
    metadata = @{type = "company"; updated = "2026-04-13"}
} | ConvertTo-Json -Compress

$temp = [System.IO.Path]::GetTempFileName() + ".json"
[System.IO.File]::WriteAllText($temp, $body)
curl.exe -s -X POST "http://localhost:8003/memory/add" -H "X-Xavier2-Token: dev-token" -H "Content-Type: application/json" --data-binary "@$temp"
Remove-Item $temp
