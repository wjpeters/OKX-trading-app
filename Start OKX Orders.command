#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy python3 scripts/start-orders.py
rtk proxy open http://127.0.0.1:4780
