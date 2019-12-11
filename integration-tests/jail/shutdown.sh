#!/bin/bash

export CURRENT_HASH=$(git rev-parse HEAD)
echo -p $CURRENT_HASH "shutdown"
