#!/bin/bash
echo "run2"

export CURRENT_HASH=$(git rev-parse HEAD)
echo "CURRENT_HASH=" $CURRENT_HASH
docker-compose -p CURRENT_HASH up -d
echo "run docker"
sleep 10

