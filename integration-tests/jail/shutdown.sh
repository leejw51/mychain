#!/bin/bash

export CURRENT_HASH=$(git rev-parse HEAD)
echo "CURRENT_HASH=" $CURRENT_HASH
echo -p $CURRENT_HASH "shutdown"
