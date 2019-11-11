#!/bin/bash
docker-compose up -d
echo "wait for docker setting up"
sleep 600
echo "done"
python3 ./disk/test.py
echo "test finished"
sleep 4
docker-compose down
echo "OK"
sleep 2
