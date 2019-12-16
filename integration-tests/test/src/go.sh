#!/bin/bash
pwd
cd /root/src
python3 ./test.py
ret=$?
echo "python result=" $ret
if [ $ret -ne 0 ]; then
    exit -1
fi
echo "OK"
