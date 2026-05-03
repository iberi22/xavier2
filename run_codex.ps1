$prompt = Get-Content 'E:\temp\p1-sevier2\PROMPT_P1.md' -Raw
$tempFile = [System.IO.Path]::GetTempFileName() + ".txt"
$prompt | Out-File -FilePath $tempFile -Encoding utf8
codex -c model="gpt-5.5" -c model_reasoning_effort="medium" --yolo exec @$tempFile 2>&1
Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
