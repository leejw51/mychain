#!/bin/bash
echo "run2"

docker-compose -p test up -d
echo "run docker"
sleep 10

