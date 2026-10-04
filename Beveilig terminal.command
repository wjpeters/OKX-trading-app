#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy python3 scripts/setup-auth.py
rtk proxy python3 scripts/start.py
