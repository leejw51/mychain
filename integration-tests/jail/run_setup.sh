#!/bin/bash
echo "setup"
sleep 1

nix-shell ./jail.nix  --run "export PASSPHRASE=1 && python3 ../bot/make.py"
cp ./node0/tendermint/config/genesis.json ./disk/config0/
cp ./node0/tendermint/config/node_key.json ./disk/config0/
cp ./node0/tendermint/config/priv_validator_key.json ./disk/config0/

cp ./node1/tendermint/config/genesis.json ./disk/config1/
cp ./node1/tendermint/config/node_key.json ./disk/config1/
cp ./node1/tendermint/config/priv_validator_key.json ./disk/config1/
