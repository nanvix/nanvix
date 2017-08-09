FROM ubuntu:xenial
ADD tools/dev /tmp/dev/
RUN apt-get update && apt-get install -y make wget tar bzip2 gzip patch && bash /tmp/dev/setup-toolchain.sh && bash /tmp/dev/setup-bochs.sh && rm -Rvf /tmp/dev
ENV TARGET i386-elf
ENV PATH $PATH:/usr/local/cross/bin
VOLUME /code
EXPOSE 1234
WORKDIR /code
ENTRYPOINT ["bash"]
