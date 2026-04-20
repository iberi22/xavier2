FROM termux/termux-docker:x86_64

# Optional prebuilt image for Termux-like smoke tests.
# If mirror selection fails during docker build, use:
#   powershell -File scripts/run_termux_docker_smoke.ps1 -RuntimeInstall
#
# The runtime-install path is slower but more reliable on Docker Desktop.

SHELL ["/data/data/com.termux/files/usr/bin/bash", "-lc"]

USER system

RUN mkdir -p "$PREFIX/tmp" \
    && pkg update -y \
    && pkg install -y python git bash coreutils findutils ripgrep \
    && pkg clean

WORKDIR /workspace

CMD ["bash"]
