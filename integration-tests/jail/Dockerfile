FROM baiduxlab/sgx-rust:1804-1.0.9
LABEL maintainer="Crypto.com"
RUN echo 'source /opt/sgxsdk/environment' >> /root/.docker_bashrc 
RUN echo 'source /root/.cargo/env' >> /root/.docker_bashrc
RUN apt-get update
ENV SGX_MODE=SW
ENV NETWORK_ID=AB
ENV RUSTFLAGS=-Ctarget-feature=+aes,+ssse3
ENV PATH=$HOME/bin:$PATH
ENV APP_PORT=25933
ENV TX_ENCLAVE_STORAGE=/enclave-storage
ENV LD_LIBRARY_PATH=$HOME/lib
ENV PKG_CONFIG_PATH=$HOME/lib/pkgconfig
CMD ["sleep","infinity"]

