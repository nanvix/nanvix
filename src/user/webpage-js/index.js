// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

const connfd = net.socket.accept();

const html = "<!DOCTYPE html><html><body><h1>Hello, World!</h1></body></html>";
const response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: " + getByteLength(html) + "\r\n\r\n" + html;

let buffer =  new TextEncoder().encode(response);

net.socket.write(connfd, buffer, 0, html.length);

function getByteLength(str) {
    const encoder = new TextEncoder();
    const encodedStr = encoder.encode(str);
    return encodedStr.length;
}
