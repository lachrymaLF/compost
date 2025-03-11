#!/bin/bash
echo "Compiling STB's image I/O library..."
gcc -c ./*.c -I ../include/ -lm
