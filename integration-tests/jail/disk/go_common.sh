#!/bin/bash
service ssh start
source /root/disk/prepare.sh
source /opt/sgxsdk/environment
source /root/.cargo/env
echo "sgx mode=" $SGX_MODE
echo "network id=" $NETWORK_ID
echo "path=" $PATH 
echo "enclave storage=" $TX_ENCLAVE_STORAGE
echo "rust flags=" $RUSTFLAGS
echo "app port=" $APP_PORT
echo "tendermint flag=" $TENDERMINT_FLAG
echo "compile chain"
