#!/bin/bash

INTERFACE="lo0"  # Change to "en0" for Wi-Fi
PACKET_LOSS="0.1"  # 10% loss
LATENCY="100ms"    # 100ms delay

enable_simulation() {
    echo "Enabling network simulation on $INTERFACE..."

    # Load dummynet module if not already enabled
    sudo sysctl -w net.inet.ip.dummynet.enabled=1

    # Enable pfctl
    sudo pfctl -E

    # Apply dummynet rules
    echo "
    dummynet in quick on $INTERFACE pipe 1
    dummynet out quick on $INTERFACE pipe 1
    " | sudo pfctl -f -

    # Configure packet loss & latency
    sudo dnctl pipe 1 config delay $LATENCY plr $PACKET_LOSS

    echo "✅ Packet loss ($PACKET_LOSS) and latency ($LATENCY) applied!"
}

disable_simulation() {
    echo "Disabling network simulation..."
    sudo pfctl -d
    sudo dnctl -q flush
    echo "✅ Network rules cleared."
}

show_rules() {
    echo "Current pfctl rules:"
    sudo pfctl -sr
    echo "Current dummynet settings:"
    sudo dnctl list
}

case "$1" in
    enable)
        enable_simulation
        ;;
    disable)
        disable_simulation
        ;;
    status)
        show_rules
        ;;
    *)
        echo "Usage: $0 {enable|disable|status}"
        exit 1
        ;;
esac

