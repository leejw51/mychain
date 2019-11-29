#!/bin/bash
. run_test_env.sh
docker-compose up -d  
echo "docker compose ok"
. /etc/profile.d/nix.sh
nix-shell ./jail.nix  --run "export PASSPHRASE=1 && python3 ../bot/jail_test.py"
ret=$?
if [ $ret -ne 0 ]; then
    exit -1
fi
echo "test finished"
sleep 4
docker-compose down
echo "OK"
sleep 2
