$env:GEMINI_CLI_TRUST_WORKSPACE = "true"
$prompt = Get-Content 'E:\temp\p3-gestalt\PROMPT_GESTALT.md' -Raw
$tempFile = [System.IO.Path]::GetTempFileName() + ".txt"
$prompt | Out-File -FilePath $tempFile -Encoding utf8
& 'C:\Users\belal\AppData\Roaming\npm\gemini.ps1' --yolo $tempFile 2>&1
Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
