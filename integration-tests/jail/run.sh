#!/bin/bash
source /etc/profile.d/nix.sh
. ./run_compile.sh

export PATH=$(pwd)/disk/bin:$PATH
. ./run_port.sh
echo "binaries"
echo $PATH
ls $(pwd)/disk/bin

echo "setup"
sleep 2
#setup
. ./run_setup.sh

echo "preparing test"
sleep 5
# test
. ./run_test.sh

echo "test finished successfully"
exit 0
