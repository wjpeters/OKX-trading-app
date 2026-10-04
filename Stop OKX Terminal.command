#!/bin/zsh
set -e
cd "${0:A:h}"
rtk proxy docker compose stop
