#!/bin/bash
. ./run_compile.sh

echo "setup"
sleep 2
#setup
. ./run_setup.sh

echo "preparing test"
sleep 5
# test
. ./run_test.sh

