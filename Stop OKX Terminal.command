#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy python3 scripts/training-profile.py stop
rtk proxy docker compose -f compose.yaml -f compose.training.yaml stop
rtk proxy python3 scripts/stop-laya.py
