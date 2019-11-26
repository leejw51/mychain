#!/bin/bash
service ssh start
cd /root/disk
cp ./config1/* /root/.tendermint/config
source ./go_common.sh
/root/disk/launch.sh
sleep infinity
