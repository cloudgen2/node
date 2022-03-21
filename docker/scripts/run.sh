#!/bin/bash

cd ~/src && nix-shell -I nixpkgs=channel:nixos-21.05 third-party/nix/shell.nix --run "cargo run --release -- --chain dev2 --tmp"
