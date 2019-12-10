#!/bin/bash
echo "run test"

. run_open_port.sh
. run_test_env.sh

echo "client rpc port="$JAIL_CLIENT_RPC
echo "chain rpc port="$JAIL_CHAIN_RPC

docker-compose up -d  
echo "docker compose ok"
nix-shell ./jail.nix  --run "export PASSPHRASE=1 && python3 ../bot/jail_test.py"

