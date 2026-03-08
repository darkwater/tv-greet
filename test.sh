#!/bin/bash

set -xe

cargo build --release
ssh sinon "sudo systemctl stop greetd"
scp target/release/tv-greet sinon:downloads/
ssh sinon "sudo systemctl restart greetd"
