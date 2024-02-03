#!/bin/sh
uftpd -n -o ftp=0,tftp=69 . &

pid=$!

minicom -D /dev/ttyACM0

kill -9 $pid
