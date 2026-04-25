$env:GROQ_API_KEY = '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'
$temp = [System.IO.Path]::GetTempFileName() + '.ogg'
Copy-Item 'C:\Users\belal\.openclaw\media\inbound\file_235---c6dcb693-6a18-4983-9b9b-4df905a6ff1b.ogg' $temp
curl.exe -s -X POST 'https://api.groq.com/openai/v1/audio/transcriptions' `
  -H "Authorization: Bearer $env:GROQ_API_KEY" `
  -F "file=@$temp" `
  -F "model=whisper-large-v3" `
  -F "language=es"
Remove-Item $temp