#!/bin/bash
export RUST_LOG=info
cd /root/bin 

echo "activate aesm"
./aesm.sh 
sleep 5

echo "activate enclave"
nohup ./enclave.sh  > enclave.log &
sleep 5

echo "activate abci"
nohup ./abci.sh  > abci.log &
sleep 30 

echo "activate tendermint"
./tendermint.sh  &
sleep 30

echo "activate client-rpc"
nohup ./client-rpc.sh > rpc.log & 
sleep 1

