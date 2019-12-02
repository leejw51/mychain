#!/bin/bash
echo "run test"
echo "client rpc port="$JAIL_CLIENT_RPC
echo "chain rpc port="$JAIL_CHAIN_RPC

. run_test_env.sh
docker-compose up -d  
echo "docker compose ok"
nix-shell ./jail.nix  --run "export PASSPHRASE=1 && python3 ../bot/jail_test.py"
ret=$?
if [ $ret -ne 0 ]; then
    docker-compose down
    exit -1
fi
echo "test finished"
sleep 4
docker-compose down
echo "OK"
sleep 2
