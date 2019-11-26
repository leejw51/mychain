#!/bin/bash
service ssh start
cd /root/disk
mkdir /root/chain
cp ./config0/* /root/.tendermint/config
source ./go_common.sh
/root/disk/launch.sh
sleep infinity
