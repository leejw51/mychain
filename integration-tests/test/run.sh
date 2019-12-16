#!/bin/bash
docker build . -t go
docker run go
ret=$?
echo "docker rsult=" $ret
if [ $ret -ne 0 ]; then
    exit -1
fi
