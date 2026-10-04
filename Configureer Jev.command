#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy python3 scripts/setup-secrets.py openrouter
rtk proxy python3 scripts/start.py
