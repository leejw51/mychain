#!/bin/bash
source /etc/profile.d/nix.sh
#. ./run_compile.sh

echo "hello"
pwd
$(pwd)/disk/bin/hello 

echo "setup"
sleep 2
#setup
#. ./run_setup.sh

echo "preparing test"
sleep 5
# test
#. ./run_test.sh

