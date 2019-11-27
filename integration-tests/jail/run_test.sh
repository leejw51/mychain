#!/bin/bash
. run_test_env.sh
docker-compose up -d  
echo "docker compose ok"
nix-shell ./jail.nix  --run "pip install python-decouple --user && python3 jail_test.py"
ret=$?
if [ $ret -ne 0 ]; then
    exit -1
fi
echo "test finished"
sleep 4
docker-compose down
echo "OK"
sleep 2
