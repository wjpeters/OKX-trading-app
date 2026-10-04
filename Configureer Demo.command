#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy python3 scripts/setup-secrets.py okx-demo
rtk proxy python3 scripts/start.py
rtk proxy python3 scripts/demo-onboard.py
