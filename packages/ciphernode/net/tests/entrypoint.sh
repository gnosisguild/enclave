#!/bin/bash
set -e

if [ "${BLOCK_MDNS:-false}" = "true" ]; then
    iptables -A INPUT -p udp --dport 5353 -j DROP
    iptables -A OUTPUT -p udp --dport 5353 -j DROP
    iptables -L | grep DROP
fi

# Execute the original command
exec "$@"
