#!/bin/bash
# install
. run_test_env.sh
#docker-compose up 
echo "docker compose ok"
python3 ./jail_test.py
ret=$?
if [ $ret -ne 0 ]; then
    exit -1
fi
echo "test finished"
sleep 4
#docker-compose down
echo "OK"
sleep 2
