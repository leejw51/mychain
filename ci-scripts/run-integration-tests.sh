#!/usr/bin/env bash
set -ex

rustup default nightly-2019-08-01-x86_64-unknown-linux-gnu

# Build test environment
cd integration-tests
./prepare.sh || exit 1
. ./env.sh
docker-compose -p "${DOCKER_COMPOSE_PREFIX}" down || (docker ps; exit 1)

# Find ports for Docker
export TENDERMINT_RPC_PORT=$(../ci-scripts/find-free-port.sh)
export CLIENT_RPC_PORT=$(../ci-scripts/find-free-port.sh)
export TENDERMINT_ZEROFEE_RPC_PORT=$(../ci-scripts/find-free-port.sh)
export CLIENT_RPC_ZEROFEE_PORT=$(../ci-scripts/find-free-port.sh)

env
cat "./${TENDERMINT_WITHFEE_DIRECTORY}/config/config.toml"
cat "./${TENDERMINT_ZEROFEE_DIRECTORY}/config/config.toml"

docker-compose -p "${DOCKER_COMPOSE_PREFIX}" up -d || (docker ps; exit 1)

./wait-for-setup.sh || (docker ps; docker-compose -p "${DOCKER_COMPOSE_PREFIX}" logs -t --tail="all"; exit 1)

# Preliminary tests
./run-test.sh || (docker ps; docker-compose -p "${DOCKER_COMPOSE_PREFIX}" logs -t --tail="all"; exit 1)

# Integration tests
cd client-rpc
npm install
npm run test || (docker ps; docker-compose -p "${DOCKER_COMPOSE_PREFIX}" logs -t --tail="all"; exit 1)
