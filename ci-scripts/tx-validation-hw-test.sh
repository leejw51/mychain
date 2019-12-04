#!/bin/bash
set -e

source /root/.docker_bashrc

export SGX_MODE=HW
export SGX_TEST=1
export NETWORK_ID=ab
export RUST_LOG=debug
export RUST_BACKTRACE=1
export RUSTFLAGS=-Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3

ls /dev/sgx

LD_LIBRARY_PATH=/opt/intel/libsgx-enclave-common/aesm /opt/intel/libsgx-enclave-common/aesm/aesm_service &

echo "[aesm_service] Running in background ..."
# Wait for aesm_service to initialize
sleep 1


cd /chain/chain-tx-enclave/tx-validation
make clean
make
cd bin
./tx-validation-app