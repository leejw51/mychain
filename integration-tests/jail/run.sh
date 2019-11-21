#!/bin/bash
docker-compose up -d --build
echo "docker compose ok"
nix-shell -p python37Packages.docker --run "python3 ./disk/jail_test.py"
ret=$?
if [ $ret -ne 0 ]; then
    exit -1
fi
echo "test finished"
sleep 4
docker-compose down
echo "OK"
sleep 2
