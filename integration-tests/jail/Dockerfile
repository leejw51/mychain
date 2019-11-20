FROM baiduxlab/sgx-rust:1804-1.0.9
LABEL maintainer="Crypto.com"

RUN echo 'source /opt/sgxsdk/environment' >> /root/.docker_bashrc 
RUN echo 'source /root/.cargo/env' >> /root/.docker_bashrc
RUN rm -rf /var/lib/apt/lists/*
RUN apt-get clean
RUN sed -i 's/ftp.daum.net/old-releases.ubuntu.com/g' /etc/apt/sources.list
RUN sed -i 's/ftp.daum.net/security.ubuntu.com/g' /etc/apt/sources.list
RUN sed -i 's/ftp.daum.net/archive.ubuntu.com/g' /etc/apt/sources.list
RUN apt-get update

CMD ["sleep","infinity"]

