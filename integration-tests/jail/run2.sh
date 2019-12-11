#!/bin/bash
echo "run2"

export CURRENT_HASH=$(git rev-parse HEAD)
docker-compose -p CURRENT_HASH up -d
echo "run docker"
sleep 10

