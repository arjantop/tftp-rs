#!/usr/bin/env bash

TFTPD_BIN=/usr/bin/in.tftpd
TFTP_BIN=/usr/bin/tftp

if [ ! -d fixtures ]; then
    echo "Run create_fixtures.sh to generate fixtures"
    exit 1
fi

if [ $# -ne 2 ]; then
    echo "Usage: $0 TRANSFER_MODE REQUEST_TYPE"
    exit 1
fi

if [ "$1" != "octet" ] && [ "$1" != "netascii" ]; then
    echo "Transfer mode must be 'octet' or 'netascii'"
    exit 1
fi

if [ "$2" != "get" ] && [ "$2" != "put" ]; then
    echo "Request time must be either 'get' or 'put'"
    exit 1
fi

for file in `ls fixtures`
do
    if [ "$2" == "get" ]; then
        echo "Getting file: [0;32m$file[0m"
        echo "Using: [0;31mtftp-hpa[0m"
        sleep 1
        /usr/bin/time "$TFTP_BIN" -v 127.0.0.1 69 -m octet -c get "$file" /tmp/testfile
        echo "Using: [0;34mtftp-rs[0m"
        sleep 1
        /usr/bin/time ../target/release/client-get "$file"
    else
        echo "Putting file: [0;32m$file[0m"
        echo "Using: [0;31mtftp-hpa[0m"
        sleep 1
        /usr/bin/time "$TFTP_BIN" -v 127.0.0.1 69 -m octet -c put "fixtures/$file" testfile
        echo "Using: [0;34mtftp-rs[0m"
        sleep 1
        /usr/bin/time ../target/release/client-put "fixtures/$file"
    fi
done
