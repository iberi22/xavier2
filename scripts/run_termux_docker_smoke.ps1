param(
    [string]$Image = "swal/termux-python:x86_64",
    [switch]$Build,
    [switch]$RuntimeInstall
)

$ErrorActionPreference = "Stop"

$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$dockerfile = Join-Path $repo "docker\termux-python\Dockerfile"

if ($Build) {
    docker build -t $Image -f $dockerfile $repo
}

if ($RuntimeInstall) {
    docker run --rm `
        -v "${repo}:/workspace" `
        -w /workspace `
        termux/termux-docker:x86_64 `
        bash -lc "mkdir -p `$PREFIX/tmp && pkg update -y && pkg install -y python git ripgrep && python scripts/termux_docker_smoke.py"
} else {
    docker run --rm `
        -v "${repo}:/workspace" `
        -w /workspace `
        $Image `
        bash -lc "python scripts/termux_docker_smoke.py"
}
