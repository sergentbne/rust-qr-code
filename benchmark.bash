#!/usr/bin/env bash

spin() {
    local pid=$1
    local delay=0.1
    local spinchars='/-\|'
    local i=0

    while kill -0 $pid 2>/dev/null; do
        local temp=${spinchars:i++%${#spinchars}:1}
        printf "\r[%s] working..." "$temp"
        sleep $delay
    done
    printf "\r"  # Clear the spinner
}

time ./target/release/rust-qr-code minimally_compressible_file.bin > /dev/null 2>&1 &
spin $!  # Call the spin function with the PID of the background command
wait $!  # Wait for the command to finish


