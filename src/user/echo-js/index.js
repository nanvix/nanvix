// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

const connfd = net.socket.accept();

const bufferSize = 1024;
const buffer = new Uint8Array(bufferSize);

net.socket.read(connfd, buffer, 0, bufferSize);

const str = new TextDecoder().decode(buffer);

net.socket.write(connfd, buffer, 0, str.length);
