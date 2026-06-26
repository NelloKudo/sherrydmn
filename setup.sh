#!/bin/bash
set -e

BINARY=/usr/local/bin/sherrydmn
SERVICE=/etc/systemd/system/sherrydmn.service

if [ "$1" = "install" ]; then
    cargo build --release
    sudo systemctl stop sherrydmn 2>/dev/null || true
    sudo cp target/release/sherrydmn "$BINARY"
    sudo cp sherrydmn.service "$SERVICE"
    sudo systemctl daemon-reload
    sudo systemctl enable --now sherrydmn
    echo "sherrydmn installed and started"

elif [ "$1" = "uninstall" ]; then
    sudo systemctl disable --now sherrydmn || true
    sudo rm -f "$BINARY" "$SERVICE"
    sudo systemctl daemon-reload
    echo "sherrydmn uninstalled"

else
    echo "usage: $0 install|uninstall"
    exit 1
fi
