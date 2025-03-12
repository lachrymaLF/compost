#!/bin/bash
cd "$(dirname "$0")"
echo "Compiling STB's image I/O library..."
gcc -c ./*.c -I ../include/ -lm
