#!/bin/bash
echo "setup"
sleep 1

python3.7 ../bot/make.py
cp ./node0/tendermint/config/* ./disk/config0/
cp ./node1/tendermint/config/* ./disk/config1/

